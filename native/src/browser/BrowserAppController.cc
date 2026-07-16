#include "browser/BrowserAppController.h"

#include <algorithm>

#include "browser/BrowserWindow.h"
#include "include/base/cef_callback.h"
#include "include/cef_parser.h"
#include "include/cef_task.h"
#include "include/wrapper/cef_closure_task.h"
#include "include/wrapper/cef_helpers.h"
#include "utils/JsonUtils.h"

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
  bool restored = false;
  if (startupBehavior == "restore") {
    restoring_ = true;
    auto windows = RestoredWindows();
    for (size_t i = 0; i < windows->GetSize(); ++i) {
      if (auto windowState = windows->GetDictionary(i)) {
        NewWindow(false, windowState);
        restored = true;
      }
    }
    restoring_ = false;
  }
  if (!restored) {
    NewWindow(false, nullptr);
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

}  // namespace

void BrowserAppController::StartHostCommandPoller() {
  CefPostDelayedTask(TID_UI, base::BindOnce(&PollHostCommands, this), 16);
}

void BrowserAppController::DispatchHostCommands() {
  std::string commandJson;
  while (engine_.PollHostCommandJson(commandJson)) {
      if (commandJson.empty()) {
        continue;
      }
      CefRefPtr<CefValue> value =
          CefParseJSON(commandJson, JSON_PARSER_ALLOW_TRAILING_COMMAS);
      if (!value || value->GetType() != VTYPE_DICTIONARY) {
        continue;
      }
      CefRefPtr<CefDictionaryValue> envelope = value->GetDictionary();
      const std::string command = envelope->HasKey("command")
                                      ? envelope->GetString("command").ToString()
                                      : "";
      const std::string commandId =
          envelope->HasKey("id") ? envelope->GetString("id").ToString() : "";
      CefRefPtr<CefDictionaryValue> payload =
          envelope->HasKey("payload") && envelope->GetType("payload") == VTYPE_DICTIONARY
              ? envelope->GetDictionary("payload")
              : CefDictionaryValue::Create();

      if (command == "window.create") {
        const bool isPrivate =
            payload->HasKey("isPrivate") && payload->GetBool("isPrivate");
        const std::string windowId = payload->GetString("windowId").ToString();
        const bool ok = !windowId.empty() && NewWindow(isPrivate, nullptr, windowId) != nullptr;
        engine_.PushHostCommandResultJson(HostCommandResultJson(
            commandId, ok, ok ? "" : "failed to create window"));
      } else if (command == "window.close") {
        const std::string targetWindowId =
            payload->HasKey("windowId") ? payload->GetString("windowId").ToString() : "";
        BrowserWindow *target = FindWindow(targetWindowId);
        const bool ok = target && target->CloseWindow();
        engine_.PushHostCommandResultJson(HostCommandResultJson(
            commandId, ok, ok ? "" : "failed to close window"));
      } else {
        BrowserWindow *target = nullptr;
        if (command == "page.create") {
          target = FindWindow(payload->GetString("windowId").ToString());
        } else if (payload->HasKey("tabId")) {
          target = FindWindowForTab(payload->GetString("tabId").ToString());
        }
        if (!target && command == "page.create") {
          engine_.PushHostCommandResultJson(
              HostCommandResultJson(commandId, false, "window not found"));
          continue;
        }
        if (!target) {
          target = activeWindow_;
        }
        if (target) {
          target->ExecuteHostCommand(commandJson);
        } else {
          engine_.PushHostCommandResultJson(
              HostCommandResultJson(commandId, false, "no target window"));
        }
      }
  }

  // Drain differential engine events every tick. Leaving this unbounded queue
  // unread caused memory growth during long sessions and meant external/core
  // mutations never reached the UI.
  std::string eventJson;
  while (engine_.PollEventJson(eventJson)) {
    auto value = CefParseJSON(eventJson, JSON_PARSER_RFC);
    if (!value || value->GetType() != VTYPE_DICTIONARY) {
      continue;
    }
    auto envelope = value->GetDictionary();
    const std::string eventName = envelope->GetString("event").ToString();
    if (eventName.empty()) {
      continue;
    }
    auto payload = envelope->HasKey("payload") &&
                           envelope->GetType("payload") == VTYPE_DICTIONARY
                       ? envelope->GetDictionary("payload")
                       : CefDictionaryValue::Create();

    // A tab move changes ownership. Route the differential event to both
    // windows so the source can remove the tab and the destination can load
    // it, even though tabId now resolves to the destination window.
    if (eventName == "tab.moved" && payload->HasKey("fromWindowId") &&
        payload->HasKey("toWindowId")) {
      const std::string fromWindowId =
          payload->GetString("fromWindowId").ToString();
      const std::string toWindowId = payload->GetString("toWindowId").ToString();
      BrowserWindow *from = FindWindow(fromWindowId);
      BrowserWindow *to = FindWindow(toWindowId);
      bool delivered = false;
      if (from && from->Bridge()) {
        from->Bridge()->EmitToUi(eventName, payload->Copy(false));
        delivered = true;
      }
      if (to && to != from && to->Bridge()) {
        to->Bridge()->EmitToUi(eventName, payload->Copy(false));
        delivered = true;
      }
      if (delivered) {
        continue;
      }
    }

    BrowserWindow *target = nullptr;
    if (payload->HasKey("windowId")) {
      target = FindWindow(payload->GetString("windowId").ToString());
    }
    if (!target && payload->HasKey("tabId")) {
      target = FindWindowForTab(payload->GetString("tabId").ToString());
    }
    if (target && target->Bridge()) {
      target->Bridge()->EmitToUi(eventName, payload->Copy(false));
      continue;
    }
    for (const auto &context : windows_) {
      if (context->window && context->window->Bridge()) {
        context->window->Bridge()->EmitToUi(eventName, payload->Copy(false));
      }
    }
  }
}

BrowserWindow *
BrowserAppController::NewWindow(bool privateWindow,
                                CefRefPtr<CefDictionaryValue> restoreState,
                                const std::string &engineWindowId) {
  CEF_REQUIRE_UI_THREAD();
  auto context = std::make_unique<WindowContext>();
  context->tabManager = std::make_unique<TabManager>(eventBus_);
  BrowserWindow *raw = nullptr;
  const std::string windowId =
      engineWindowId.empty() ? NextWindowId() : engineWindowId;
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
  if (!activeWindow_) {
    return false;
  }
  auto value = activeWindow_->Bridge()->Invoke(
      "windows.reopenClosed", CefDictionaryValue::Create());
  return value && value->GetType() == VTYPE_BOOL && value->GetBool();
}

void BrowserAppController::NotifyWindowFocused(BrowserWindow *window) {
  activeWindow_ = window;
  if (window) {
    eventBus_.Publish({EventType::WindowFocused,
                       "window.focused",
                       {},
                       window->WindowId(),
                       "",
                       ""});
    auto root = CefDictionaryValue::Create();
    root->SetInt("version", 0);
    root->SetString("event", "window.focused");
    auto payload = CefDictionaryValue::Create();
    payload->SetString("windowId", window->WindowId());
    root->SetDictionary("payload", payload);
    auto value = CefValue::Create();
    value->SetDictionary(root);
    engine_.PushHostEventJson(CefWriteJSON(value, JSON_WRITER_DEFAULT).ToString());
  }
}

void BrowserAppController::NotifyWindowClosed(BrowserWindow *window) {
  CEF_REQUIRE_UI_THREAD();
  if (!window) {
    return;
  }
  // NSWindow invokes windowWillClose synchronously from performClose:. Erasing
  // the owning unique_ptr in that callback destroyed BrowserWindow while its
  // CloseWindow() stack frame was still active. Defer ownership teardown until
  // Cocoa has unwound the close callback.
  CefPostTask(TID_UI, base::BindOnce(
                          [](BrowserAppController *app, BrowserWindow *closed) {
                            if (app && app == GetBrowserAppController()) {
                              app->FinalizeWindowClosed(closed);
                            }
                          },
                          this, window));
}

void BrowserAppController::FinalizeWindowClosed(BrowserWindow *window) {
  CEF_REQUIRE_UI_THREAD();
  if (!window) {
    return;
  }
  const auto existing = std::find_if(
      windows_.begin(), windows_.end(),
      [&](const std::unique_ptr<WindowContext> &context) {
        return context->window.get() == window;
      });
  if (existing == windows_.end()) {
    return;
  }
  const std::string windowId = window->WindowId();
  // Notify FrostEngine before removing the window
  auto root = CefDictionaryValue::Create();
  root->SetInt("version", 0);
  root->SetString("event", "window.closed");
  auto payload = CefDictionaryValue::Create();
  payload->SetString("windowId", windowId);
  root->SetDictionary("payload", payload);
  auto value = CefValue::Create();
  value->SetDictionary(root);
  engine_.PushHostEventJson(CefWriteJSON(value, JSON_WRITER_DEFAULT).ToString());
  windows_.erase(existing);
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

BrowserWindow *BrowserAppController::FindWindow(const std::string &windowId) const {
  for (const auto &context : windows_) {
    if (context->window && context->window->WindowId() == windowId) {
      return context->window.get();
    }
  }
  return nullptr;
}

BrowserWindow *BrowserAppController::FindWindowForTab(const std::string &tabId) const {
  for (const auto &context : windows_) {
    if (!context->window) {
      continue;
    }
    for (const auto &tab : context->window->Tabs().GetTabs()) {
      if (tab.id == tabId) {
        return context->window.get();
      }
    }
  }
  return nullptr;
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
