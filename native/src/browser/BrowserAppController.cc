#include "browser/BrowserAppController.h"

#include <algorithm>

#include "browser/BrowserWindow.h"
#include "include/base/cef_callback.h"
#include "include/cef_parser.h"
#include "include/cef_task.h"
#include "include/wrapper/cef_closure_task.h"
#include "include/wrapper/cef_helpers.h"

namespace fubuki {

namespace {

BrowserAppController *gController = nullptr;

CefRefPtr<CefValue>
ValueFromDictionary(CefRefPtr<CefDictionaryValue> dictionary) {
  auto value = CefValue::Create();
  value->SetDictionary(dictionary);
  return value;
}

}  // namespace

BrowserAppController::BrowserAppController(std::filesystem::path profilePath)
    : profilePath_(std::move(profilePath)),
      engine_(profilePath_.string() + "/frost-engine.sqlite3"),
      store_(profilePath_, engine_.RawHandle()) {
  store_.AddLog("info", "BrowserAppController initialized");
}

BrowserAppController::~BrowserAppController() = default;

void BrowserAppController::Start() {
  CEF_REQUIRE_UI_THREAD();
  const std::string startupBehavior = store_.GetSetting("startupBehavior");
  const std::string engineWindowId = EngineWindowId();
  bool restored = false;
  if (startupBehavior == "restore") {
    restoring_ = true;
    auto windows = RestoredWindows();
    for (size_t i = 0; i < windows->GetSize(); ++i) {
      if (auto windowState = windows->GetDictionary(i)) {
        NewWindow(false, windowState, i == 0 ? engineWindowId : "");
        restored = true;
      }
    }
    restoring_ = false;
  }
  if (!restored) {
    NewWindow(false, nullptr, engineWindowId);
  }
  StartHostCommandPoller();
}

namespace {

// Self-rescheduling host command poller. Runs on the CEF UI thread and drains
// FrostEngine HostCommands at a fixed cadence. The poller verifies the
// controller is still the current instance before rescheduling, preventing a
// use-after-free if the controller is destroyed between ticks.
void PollHostCommands(BrowserAppController *app) {
  if (app && app == GetBrowserAppController()) {
    app->DispatchHostCommands();
    CefPostDelayedTask(TID_UI, base::BindOnce(&PollHostCommands, app), 16);
  }
}

// Builds a HostCommandResult JSON envelope for the given command id.
std::string HostCommandResultJson(const std::string &commandId, bool ok,
                                  const std::string &error) {
  auto root = CefDictionaryValue::Create();
  root->SetInt("version", 0);
  root->SetString("commandId", commandId);
  root->SetBool("ok", ok);
  if (!ok) {
    root->SetString("error", error);
  }
  auto value = CefValue::Create();
  value->SetDictionary(root);
  return CefWriteJSON(value, JSON_WRITER_DEFAULT).ToString();
}

std::string HostWindowEventJson(const std::string &event,
                                const std::string &windowId) {
  auto root = CefDictionaryValue::Create();
  auto payload = CefDictionaryValue::Create();
  root->SetInt("version", 0);
  root->SetString("event", event);
  payload->SetString("windowId", windowId);
  root->SetDictionary("payload", payload);
  auto value = CefValue::Create();
  value->SetDictionary(root);
  return CefWriteJSON(value, JSON_WRITER_DEFAULT).ToString();
}

}  // namespace

void BrowserAppController::StartHostCommandPoller() {
  CefPostDelayedTask(TID_UI, base::BindOnce(&PollHostCommands, this), 16);
}

void BrowserAppController::DispatchHostCommands() {
  std::string commandJson;
  while (engine_.PollHostCommandJson(commandJson)) {
    CefRefPtr<CefValue> value =
        CefParseJSON(commandJson, JSON_PARSER_ALLOW_TRAILING_COMMAS);
    if (!value || value->GetType() != VTYPE_DICTIONARY) {
      LOG(ERROR) << "[FrostHost] Dropped malformed host command";
      continue;
    }

    CefRefPtr<CefDictionaryValue> envelope = value->GetDictionary();
    const std::string command = envelope->HasKey("command")
                                    ? envelope->GetString("command").ToString()
                                    : "";
    const std::string commandId =
        envelope->HasKey("id") ? envelope->GetString("id").ToString() : "";
    CefRefPtr<CefDictionaryValue> payload =
        envelope->HasKey("payload") &&
                envelope->GetType("payload") == VTYPE_DICTIONARY
            ? envelope->GetDictionary("payload")
            : CefDictionaryValue::Create();

    auto findWindow = [this](const std::string &windowId) -> BrowserWindow * {
      for (const auto &context : windows_) {
        if (context->window && context->window->WindowId() == windowId) {
          return context->window.get();
        }
      }
      return nullptr;
    };
    auto findTabWindow = [this](const std::string &tabId) -> BrowserWindow * {
      for (const auto &context : windows_) {
        if (context->window && context->window->Tabs().GetTab(tabId)) {
          return context->window.get();
        }
      }
      return nullptr;
    };

    if (command == "window.create") {
      const std::string windowId = payload->GetString("windowId").ToString();
      const bool isPrivate = payload->HasKey("isPrivate") &&
                             payload->GetBool("isPrivate");
      const bool ok = !windowId.empty() && !findWindow(windowId) &&
                      NewWindow(isPrivate, nullptr, windowId) != nullptr;
      engine_.PushHostCommandResultJson(HostCommandResultJson(
          commandId, ok, ok ? "" : "failed to create target window"));
      continue;
    }

    if (command == "window.close") {
      const std::string windowId = payload->GetString("windowId").ToString();
      BrowserWindow *target = findWindow(windowId);
      const bool ok = target && target->CloseWindow();
      engine_.PushHostCommandResultJson(HostCommandResultJson(
          commandId, ok, ok ? "" : "target window was not available"));
      continue;
    }

    const std::string windowId = payload->GetString("windowId").ToString();
    const std::string tabId = payload->GetString("tabId").ToString();
    BrowserWindow *target = !windowId.empty() ? findWindow(windowId)
                            : !tabId.empty() ? findTabWindow(tabId)
                                             : activeWindow_;
    if (target) {
      target->ExecuteHostCommand(commandJson);
    } else {
      engine_.PushHostCommandResultJson(HostCommandResultJson(
          commandId, false, "host command target was not available"));
    }
  }
}

BrowserWindow *
BrowserAppController::NewWindow(bool privateWindow,
                                CefRefPtr<CefDictionaryValue> restoreState,
                                const std::string &requestedWindowId) {
  CEF_REQUIRE_UI_THREAD();
  auto context = std::make_unique<WindowContext>();
  context->tabManager = std::make_unique<TabManager>(eventBus_);
  BrowserWindow *raw = nullptr;
  const std::string windowId =
      requestedWindowId.empty() ? NextWindowId() : requestedWindowId;
  for (const auto &existing : windows_) {
    if (existing->window && existing->window->WindowId() == windowId) {
      LOG(ERROR) << "[FrostHost] Refused duplicate window id " << windowId;
      return nullptr;
    }
  }
  context->window = std::make_unique<BrowserWindow>(*this, *context->tabManager,
                                                    windowId, privateWindow);
  raw = context->window.get();
  windows_.push_back(std::move(context));
  activeWindow_ = raw;
  raw->Show(restoreState);
  eventBus_.Publish({EventType::WindowCreated,
                     "window.created",
                     {},
                     windowId,
                     "",
                     privateWindow ? "private" : "normal"});
  if (!privateWindow && !restoring_) {
    PersistSession();
  }
  return raw;
}

bool BrowserAppController::NewPrivateWindow() {
  return NewWindow(true, nullptr) != nullptr;
}

bool BrowserAppController::RequestNewWindow(
    bool privateWindow, CefRefPtr<CefDictionaryValue> restoreState) {
  if (!restoreState) {
    const std::string method =
        privateWindow ? "windows.createPrivate" : "windows.create";
    const std::string request = "{\"version\":0,\"method\":\"" + method +
                                "\",\"params\":{}}";
    CefRefPtr<CefValue> response =
        CefParseJSON(engine_.ProcessJson(request), JSON_PARSER_RFC);
    return response && response->GetType() == VTYPE_DICTIONARY &&
           response->GetDictionary()->GetBool("ok");
  }
  CefPostTask(TID_UI, base::BindOnce(
                          [](BrowserAppController *app, bool privateWindow,
                             CefRefPtr<CefDictionaryValue> restoreState) {
                            if (app) {
                              app->NewWindow(privateWindow, restoreState);
                            }
                          },
                          this, privateWindow, restoreState));
  return true;
}

bool BrowserAppController::RequestNewPrivateWindow() {
  return RequestNewWindow(true, nullptr);
}

bool BrowserAppController::CloseActiveWindow() {
  if (!activeWindow_) {
    return false;
  }
  return activeWindow_->CloseWindow();
}

bool BrowserAppController::ReopenClosedWindow() {
  if (closedWindows_.empty()) {
    return false;
  }
  auto state = closedWindows_.back();
  closedWindows_.pop_back();
  NewWindow(false, state);
  return true;
}

void BrowserAppController::NotifyWindowFocused(BrowserWindow *window) {
  activeWindow_ = window;
  if (window) {
    engine_.PushHostEventJson(
        HostWindowEventJson("window.focused", window->WindowId()));
    eventBus_.Publish({EventType::WindowFocused,
                       "window.focused",
                       {},
                       window->WindowId(),
                       "",
                       ""});
  }
}

void BrowserAppController::NotifyWindowClosed(BrowserWindow *window) {
  CEF_REQUIRE_UI_THREAD();
  if (!window) {
    return;
  }
  const std::string windowId = window->WindowId();
  engine_.PushHostEventJson(HostWindowEventJson("window.closed", windowId));
  if (!window->IsPrivate()) {
    closedWindows_.push_back(window->SessionSnapshot());
    if (closedWindows_.size() > 10) {
      closedWindows_.erase(closedWindows_.begin());
    }
  }
  auto it = std::find_if(windows_.begin(), windows_.end(),
                         [&](const std::unique_ptr<WindowContext> &context) {
                           return context->window.get() == window;
                         });
  if (it != windows_.end()) {
    windows_.erase(it);
  }
  activeWindow_ = windows_.empty() ? nullptr : windows_.back()->window.get();
  eventBus_.Publish(
      {EventType::WindowClosed, "window.closed", {}, windowId, "", ""});
  PersistSession();
}

void BrowserAppController::PersistSession() {
  CEF_REQUIRE_UI_THREAD();
  auto root = CefDictionaryValue::Create();
  auto windows = CefListValue::Create();
  size_t index = 0;
  for (const auto &context : windows_) {
    if (!context->window || context->window->IsPrivate()) {
      continue;
    }
    windows->SetDictionary(index++, context->window->SessionSnapshot());
  }
  root->SetInt("version", 1);
  root->SetList("windows", windows);
  store_.SetSetting(
      "sessionJson",
      CefWriteJSON(ValueFromDictionary(root), JSON_WRITER_DEFAULT).ToString());
}

BrowserWindow *BrowserAppController::ActiveWindow() const {
  return activeWindow_;
}

std::vector<BrowserWindow *> BrowserAppController::Windows() const {
  std::vector<BrowserWindow *> result;
  for (const auto &context : windows_) {
    if (context->window) {
      result.push_back(context->window.get());
    }
  }
  return result;
}

std::string BrowserAppController::NextWindowId() {
  return "window-" + std::to_string(nextWindowId_++);
}

std::string BrowserAppController::EngineWindowId() {
  const std::string response = engine_.ProcessJson(
      R"({"version":0,"method":"app.snapshot","params":{}})");
  CefRefPtr<CefValue> value = CefParseJSON(response, JSON_PARSER_RFC);
  if (!value || value->GetType() != VTYPE_DICTIONARY) {
    return NextWindowId();
  }
  CefRefPtr<CefDictionaryValue> envelope = value->GetDictionary();
  if (!envelope->HasKey("result") ||
      envelope->GetType("result") != VTYPE_DICTIONARY) {
    return NextWindowId();
  }
  CefRefPtr<CefDictionaryValue> state = envelope->GetDictionary("result");
  const std::string activeWindowId =
      state->GetString("activeWindowId").ToString();
  if (!activeWindowId.empty()) {
    return activeWindowId;
  }
  if (state->HasKey("windows") && state->GetType("windows") == VTYPE_LIST) {
    CefRefPtr<CefListValue> windows = state->GetList("windows");
    if (windows && windows->GetSize() > 0) {
      CefRefPtr<CefDictionaryValue> window = windows->GetDictionary(0);
      if (window) {
        const std::string id = window->GetString("id").ToString();
        if (!id.empty()) {
          return id;
        }
      }
    }
  }
  return NextWindowId();
}

CefRefPtr<CefListValue> BrowserAppController::RestoredWindows() const {
  auto empty = CefListValue::Create();
  const std::string json = store_.GetSetting("sessionJson");
  if (json.empty()) {
    return empty;
  }
  auto parsed = CefParseJSON(json, JSON_PARSER_RFC);
  if (!parsed || parsed->GetType() != VTYPE_DICTIONARY) {
    return empty;
  }
  auto root = parsed->GetDictionary();
  if (!root || !root->HasKey("windows") ||
      root->GetType("windows") != VTYPE_LIST) {
    return empty;
  }
  return root->GetList("windows");
}

BrowserAppController *GetBrowserAppController() {
  return gController;
}

void SetBrowserAppController(BrowserAppController *controller) {
  gController = controller;
}

bool DispatchBrowserMenuCommand(const std::string &commandId) {
  BrowserAppController *app = GetBrowserAppController();
  BrowserWindow *window = app ? app->ActiveWindow() : nullptr;
  if (!app) {
    return false;
  }
  if (commandId == "windows.create") {
    return app->RequestNewWindow(false, nullptr);
  }
  if (commandId == "windows.createPrivate") {
    return app->RequestNewPrivateWindow();
  }
  if (commandId == "windows.close") {
    return app->CloseActiveWindow();
  }
  if (commandId == "windows.reopenClosed") {
    return app->ReopenClosedWindow();
  }
  if (!window &&
      (commandId == "tabs.create" || commandId == "app.openDownloads" ||
       commandId == "app.openHistory" || commandId == "app.openBookmarks" ||
       commandId == "app.openSettings" || commandId == "app.openDebug" ||
       commandId == "app.toggleSidebar")) {
    window = app->NewWindow(false, nullptr);
  }
  if (!window) {
    return false;
  }
  auto result = window->ExecuteCommand(commandId, CefDictionaryValue::Create());
  if (!result || result->GetType() == VTYPE_NULL) {
    return false;
  }
  return result->GetType() == VTYPE_BOOL ? result->GetBool() : true;
}

}  // namespace fubuki
