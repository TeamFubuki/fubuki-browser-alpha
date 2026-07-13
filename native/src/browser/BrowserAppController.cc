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

BrowserAppController::BrowserAppController(std::filesystem::path profilePath,
                                           std::filesystem::path uiResourcesPath)
    : profilePath_(std::move(profilePath)),
      uiResourcesPath_(std::move(uiResourcesPath)),
      engine_(profilePath_.string() + "/frost-engine.sqlite3"),
      store_(profilePath_, engine_.RawHandle()) {
  store_.AddLog("info", "BrowserAppController initialized");
}

BrowserAppController::~BrowserAppController() = default;

void BrowserAppController::Start() {
  CEF_REQUIRE_UI_THREAD();
  const std::string snapshotJson = engine_.ProcessJson(
      R"({"version":0,"method":"app.snapshot","params":{}})");
  CefRefPtr<CefValue> parsed = CefParseJSON(snapshotJson, JSON_PARSER_RFC);
  if (!parsed || parsed->GetType() != VTYPE_DICTIONARY ||
      !parsed->GetDictionary()->GetBool("ok")) {
    LOG(FATAL) << "FrostEngine failed to provide the startup snapshot";
    return;
  }
  CefRefPtr<CefDictionaryValue> state =
      parsed->GetDictionary()->GetDictionary("result");
  CefRefPtr<CefListValue> engineWindows = state->GetList("windows");
  CefRefPtr<CefListValue> engineTabs = state->GetList("tabs");
  if (!engineWindows) {
    LOG(FATAL) << "FrostEngine startup snapshot has no windows";
    return;
  }

  restoring_ = true;
  for (size_t windowIndex = 0; windowIndex < engineWindows->GetSize();
       ++windowIndex) {
    CefRefPtr<CefDictionaryValue> engineWindow =
        engineWindows->GetDictionary(windowIndex);
    if (!engineWindow) {
      continue;
    }
    auto restoreState = CefDictionaryValue::Create();
    restoreState->SetBool("engineOwned", true);
    auto tabs = CefListValue::Create();
    size_t tabIndex = 0;
    if (engineTabs) {
      for (size_t index = 0; index < engineTabs->GetSize(); ++index) {
        CefRefPtr<CefDictionaryValue> tab = engineTabs->GetDictionary(index);
        if (tab && tab->GetString("windowId") == engineWindow->GetString("id")) {
          auto restoredTab = CefDictionaryValue::Create();
          restoredTab->SetString("id", tab->GetString("id"));
          restoredTab->SetString("title", tab->GetString("title"));
          restoredTab->SetString("url", tab->GetString("url"));
          restoredTab->SetString("faviconUrl", tab->GetString("faviconUrl"));
          restoredTab->SetBool("active", tab->GetBool("isActive"));
          restoredTab->SetBool("pinned", tab->GetBool("isPinned"));
          tabs->SetDictionary(tabIndex++, restoredTab);
        }
      }
    }
    restoreState->SetList("tabs", tabs);
    NewWindow(engineWindow->GetBool("isPrivate"), restoreState,
              engineWindow->GetString("id").ToString());
  }
  restoring_ = false;
  if (windows_.empty()) {
    LOG(FATAL) << "FrostEngine startup snapshot produced no usable windows";
    return;
  }
  if (!engineTabs || engineTabs->GetSize() == 0) {
    const std::string createResponse = engine_.ProcessJson(
        R"({"version":0,"method":"tabs.create","params":{"active":true}})");
    CefRefPtr<CefValue> create = CefParseJSON(createResponse, JSON_PARSER_RFC);
    if (!create || create->GetType() != VTYPE_DICTIONARY ||
        !create->GetDictionary()->GetBool("ok")) {
      LOG(FATAL) << "FrostEngine failed to create the initial tab";
      return;
    }
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
    app->DispatchEngineEvents();
    app->ReportHostState();
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
      LOG(ERROR) << "[HostCommand] Invalid command envelope";
      continue;
    }
    CefRefPtr<CefDictionaryValue> envelope = value->GetDictionary();
    const std::string command = envelope->GetString("command").ToString();
    const std::string commandId = envelope->GetString("id").ToString();
    CefRefPtr<CefDictionaryValue> payload =
        envelope->HasKey("payload") &&
                envelope->GetType("payload") == VTYPE_DICTIONARY
            ? envelope->GetDictionary("payload")
            : CefDictionaryValue::Create();

    if (command == "window.create") {
      const bool isPrivate =
          payload->HasKey("isPrivate") && payload->GetBool("isPrivate");
      const std::string windowId = payload->GetString("windowId").ToString();
      auto restoreState = CefDictionaryValue::Create();
      restoreState->SetBool("engineOwned", true);
      restoreState->SetList("tabs", CefListValue::Create());
      const bool ok = !windowId.empty() &&
                      NewWindow(isPrivate, restoreState, windowId) != nullptr;
      engine_.PushHostCommandResultJson(HostCommandResultJson(
          commandId, ok, ok ? "" : "failed to create window"));
      continue;
    }

    if (command == "window.close") {
      const std::string windowId = payload->GetString("windowId").ToString();
      auto it = std::find_if(windows_.begin(), windows_.end(),
                             [&windowId](const auto &context) {
                               return context->window &&
                                      context->window->WindowId() == windowId;
                             });
      const bool ok = it != windows_.end() && (*it)->window->CloseWindow();
      engine_.PushHostCommandResultJson(HostCommandResultJson(
          commandId, ok, ok ? "" : "unknown or uncloseable window"));
      continue;
    }

    const std::string tabId = payload->GetString("tabId").ToString();
    const std::string windowId = payload->GetString("windowId").ToString();
    BrowserWindow *target = nullptr;
    for (const auto &context : windows_) {
      BrowserWindow *candidate = context->window.get();
      if (candidate && ((!windowId.empty() && candidate->WindowId() == windowId) ||
                        (!tabId.empty() && candidate->Tabs().GetTab(tabId)))) {
        target = candidate;
        break;
      }
    }
    if (!target && tabId.empty() && windowId.empty()) {
      target = activeWindow_;
    }
    if (target) {
      target->ExecuteHostCommand(commandJson);
    } else {
      engine_.PushHostCommandResultJson(HostCommandResultJson(
          commandId, false, "host command target not found"));
    }
  }
}

void BrowserAppController::DispatchEngineEvents() {
  std::string eventJson;
  while (engine_.PollEventJson(eventJson)) {
    CefRefPtr<CefValue> value = CefParseJSON(eventJson, JSON_PARSER_RFC);
    if (!value || value->GetType() != VTYPE_DICTIONARY) {
      LOG(ERROR) << "[EngineEvent] Invalid event envelope";
      continue;
    }
    CefRefPtr<CefDictionaryValue> envelope = value->GetDictionary();
    if (envelope->GetInt("version") != 0 ||
        envelope->GetType("event") != VTYPE_STRING) {
      LOG(ERROR) << "[EngineEvent] Invalid version or event name";
      continue;
    }
    const std::string eventName = envelope->GetString("event").ToString();
    CefRefPtr<CefDictionaryValue> payload =
        envelope->GetType("payload") == VTYPE_DICTIONARY
            ? envelope->GetDictionary("payload")
            : CefDictionaryValue::Create();
    for (const auto &context : windows_) {
      if (context->window && context->window->Bridge()) {
        context->window->Bridge()->EmitToUi(eventName, payload->Copy(false));
        if (eventName == "host.commandFailed") {
          context->window->Bridge()->EmitToUi(
              "app.stateChanged", CefDictionaryValue::Create());
        }
      }
    }
  }
}

void BrowserAppController::ReportHostState() {
  const auto now = std::chrono::steady_clock::now();
  if (now < nextStateReport_) {
    return;
  }
  nextStateReport_ = now + std::chrono::seconds(1);
  auto event = CefDictionaryValue::Create();
  event->SetInt("version", 0);
  event->SetString("event", "host.stateObserved");
  auto payload = CefDictionaryValue::Create();
  auto windowIds = CefListValue::Create();
  auto tabIds = CefListValue::Create();
  size_t windowIndex = 0;
  size_t tabIndex = 0;
  for (const auto &context : windows_) {
    if (!context->window) {
      continue;
    }
    windowIds->SetString(windowIndex++, context->window->WindowId());
    for (const Tab &tab : context->window->Tabs().GetTabs()) {
      tabIds->SetString(tabIndex++, tab.id);
    }
  }
  payload->SetList("windowIds", windowIds);
  payload->SetList("tabIds", tabIds);
  event->SetDictionary("payload", payload);
  if (!engine_.PushHostEventJson(
          CefWriteJSON(ValueFromDictionary(event), JSON_WRITER_DEFAULT)
              .ToString())) {
    LOG(ERROR) << "Failed to report Host state to FrostEngine";
  }
}

BrowserWindow *
BrowserAppController::NewWindow(bool privateWindow,
                                CefRefPtr<CefDictionaryValue> restoreState,
                                std::string engineWindowId) {
  CEF_REQUIRE_UI_THREAD();
  auto context = std::make_unique<WindowContext>();
  context->tabManager = std::make_unique<TabManager>(eventBus_);
  BrowserWindow *raw = nullptr;
  const std::string windowId =
      engineWindowId.empty() ? NextWindowId() : std::move(engineWindowId);
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
  return raw;
}

bool BrowserAppController::NewPrivateWindow() {
  return NewWindow(true, nullptr) != nullptr;
}

bool BrowserAppController::RequestNewWindow(
    bool privateWindow, CefRefPtr<CefDictionaryValue>) {
  const std::string method =
      privateWindow ? "windows.createPrivate" : "windows.create";
  CefRefPtr<CefValue> response = CefParseJSON(
      engine_.ProcessJson("{\"version\":0,\"method\":\"" + method +
                          "\",\"params\":{}}"),
      JSON_PARSER_RFC);
  return response && response->GetType() == VTYPE_DICTIONARY &&
         response->GetDictionary()->GetBool("ok");
}

bool BrowserAppController::RequestNewPrivateWindow() {
  return RequestNewWindow(true, nullptr);
}

bool BrowserAppController::CloseActiveWindow() {
  if (!activeWindow_) {
    return false;
  }
  CefRefPtr<CefValue> response = CefParseJSON(
      engine_.ProcessJson(
          "{\"version\":0,\"method\":\"windows.close\",\"params\":{\"windowId\":\"" +
          activeWindow_->WindowId() + "\"}}"),
      JSON_PARSER_RFC);
  return response && response->GetType() == VTYPE_DICTIONARY &&
         response->GetDictionary()->GetBool("ok");
}

bool BrowserAppController::ReopenClosedWindow() {
  CefRefPtr<CefValue> response = CefParseJSON(
      engine_.ProcessJson(
          R"({"version":0,"method":"windows.reopenClosed","params":{}})"),
      JSON_PARSER_RFC);
  return response && response->GetType() == VTYPE_DICTIONARY &&
         response->GetDictionary()->GetBool("ok");
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
  }
}

void BrowserAppController::NotifyWindowClosed(BrowserWindow *window) {
  CEF_REQUIRE_UI_THREAD();
  if (!window) {
    return;
  }
  const std::string windowId = window->WindowId();
  if (!shuttingDown_) {
    auto hostEvent = CefDictionaryValue::Create();
    hostEvent->SetInt("version", 0);
    hostEvent->SetString("event", "window.closed");
    auto payload = CefDictionaryValue::Create();
    payload->SetString("windowId", windowId);
    hostEvent->SetDictionary("payload", payload);
    engine_.PushHostEventJson(CefWriteJSON(ValueFromDictionary(hostEvent),
                                           JSON_WRITER_DEFAULT)
                                  .ToString());
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
  if (!window) {
    if (commandId != "windows.create" &&
        commandId != "windows.createPrivate" &&
        commandId != "windows.reopenClosed") {
      return false;
    }
    const std::string response = app->Engine().ProcessJson(
        "{\"version\":0,\"method\":\"" + commandId +
        "\",\"params\":{}}");
    CefRefPtr<CefValue> parsed = CefParseJSON(response, JSON_PARSER_RFC);
    return parsed && parsed->GetType() == VTYPE_DICTIONARY &&
           parsed->GetDictionary()->GetBool("ok");
  }
  auto params = CefDictionaryValue::Create();
  params->SetString("id", commandId);
  params->SetDictionary("args", CefDictionaryValue::Create());
  auto result = window->Bridge()->Invoke("commands.execute", params);
  if (!result || result->GetType() == VTYPE_NULL) {
    return false;
  }
  return result->GetType() == VTYPE_BOOL ? result->GetBool() : true;
}

}  // namespace fubuki
