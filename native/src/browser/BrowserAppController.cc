#include "browser/BrowserAppController.h"

#include <algorithm>
#include <chrono>

#include "browser/BrowserWindow.h"
#include "include/base/cef_callback.h"
#include "include/cef_parser.h"
#include "include/cef_task.h"
#include "include/wrapper/cef_closure_task.h"
#include "include/wrapper/cef_helpers.h"

namespace fubuki {

namespace {

BrowserAppController *gController = nullptr;

void OnHostCommandReady(void *context) {
  auto *callbackContext = static_cast<HostCommandCallbackContext *>(context);
  auto *app = callbackContext ? callbackContext->app.load() : nullptr;
  if (app) {
    app->NotifyHostCommandReady();
  }
}

}  // namespace

BrowserAppController::BrowserAppController(std::filesystem::path profilePath)
    : profilePath_(std::move(profilePath)),
      engine_(profilePath_.string() + "/frost-engine.sqlite3"),
      store_(profilePath_, engine_.RawHandle()) {
  hostCommandCallbackContext_.app.store(this);
  engine_.SetHostCommandNotifier(&OnHostCommandReady,
                                 &hostCommandCallbackContext_);
  store_.AddLog("info", "BrowserAppController initialized");
}

BrowserAppController::~BrowserAppController() {
  // Async callbacks may reference the controller and its store. Stop and join
  // the engine request worker while all dependent members are still alive.
  if (GetBrowserAppController() == this) {
    SetBrowserAppController(nullptr);
  }
  hostCommandCallbackContext_.app.store(nullptr);
  engine_.SetHostCommandNotifier(nullptr, nullptr);
  engine_.ShutdownAsync();
}

void BrowserAppController::Start() {
  CEF_REQUIRE_UI_THREAD();
  // Bootstrap through FrostEngine, not by manufacturing native state. The
  // resulting window/page commands are consumed by the shared dispatcher.
  // windows.create now automatically creates a startup tab, so we only
  // need a single InvokeEngine call.
  if (!InvokeEngine("app.startup")) {
    store_.AddLog("error", "FrostEngine failed to queue the initial browser state");
  }
}

namespace {

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

void BrowserAppController::DispatchEngineEvents() {
  std::string eventJson;
  while (engine_.PollEventJson(eventJson)) {
    CefRefPtr<CefValue> value =
        CefParseJSON(eventJson, JSON_PARSER_ALLOW_TRAILING_COMMAS);
    if (!value || value->GetType() != VTYPE_DICTIONARY) {
      continue;
    }
    auto envelope = value->GetDictionary();
    if (!envelope->HasKey("event") ||
        envelope->GetType("event") != VTYPE_STRING ||
        !envelope->HasKey("payload") ||
        envelope->GetType("payload") != VTYPE_DICTIONARY) {
      continue;
    }
    const std::string eventName = envelope->GetString("event").ToString();
    // Settings are global, so broadcast them to every UI bridge. Other
    // lifecycle events are already emitted by the host-side event bus.
    if (eventName != "setting.changed") {
      continue;
    }
    auto payload = envelope->GetDictionary("payload");
    for (const auto &context : windows_) {
      if (context && context->window && context->window->Bridge()) {
        context->window->Bridge()->EmitToUi(eventName, payload->Copy(false));
      }
    }
  }
}

void BrowserAppController::NotifyHostCommandReady() {
  if (hostDispatchScheduled_.exchange(true)) {
    return;
  }
  CefPostTask(TID_UI, base::BindOnce([](BrowserAppController *app) {
    if (app && app == GetBrowserAppController()) {
      app->hostDispatchScheduled_.store(false);
      app->DispatchHostCommands();
    }
  }, this));
}

void BrowserAppController::DispatchHostCommands() {
  CEF_REQUIRE_UI_THREAD();
  constexpr size_t kMaxCommandsPerDrain = 32;
  constexpr auto kMaxDrainTime = std::chrono::milliseconds(4);
  const auto started = std::chrono::steady_clock::now();
  size_t processed = 0;
  std::string commandJson;
  // There is one FrostBridge and one queue. Poll it exactly once here, then
  // route each command to the window identified by its payload.
  while (processed < kMaxCommandsPerDrain &&
         std::chrono::steady_clock::now() - started < kMaxDrainTime &&
         engine_.PollHostCommandJson(commandJson)) {
      ++processed;
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
          envelope->HasKey("payload") &&
                  envelope->GetType("payload") == VTYPE_DICTIONARY
              ? envelope->GetDictionary("payload")
              : CefDictionaryValue::Create();

      if (command == "window.create") {
        const bool isPrivate =
            payload->HasKey("isPrivate") && payload->GetBool("isPrivate");
        const std::string windowId = payload->HasKey("windowId")
                                         ? payload->GetString("windowId").ToString()
                                         : "";
        const bool ok = !windowId.empty() && !FindWindow(windowId) &&
                        NewWindow(isPrivate, nullptr, windowId) != nullptr;
        engine_.PushHostCommandResultJson(HostCommandResultJson(
            commandId, ok, ok ? "" : "failed to create requested window"));
      } else if (command == "window.close") {
        const std::string targetWindowId =
            payload->HasKey("windowId") ? payload->GetString("windowId").ToString() : "";
        BrowserWindow *target = FindWindow(targetWindowId);
        const bool ok = target && target->CloseWindow();
        engine_.PushHostCommandResultJson(HostCommandResultJson(
            commandId, ok, ok ? "" : "target window does not exist or could not close"));
      } else if (command == "settings.apply") {
        const std::string key = payload->HasKey("key")
                                    ? payload->GetString("key").ToString()
                                    : "";
        const std::string settingValue = payload->HasKey("value")
                                             ? payload->GetString("value").ToString()
                                             : "";
        bool ok = !key.empty();
        for (const auto &context : windows_) {
          ok = context && context->window &&
               context->window->ApplySetting(key, settingValue) && ok;
        }
        engine_.PushHostCommandResultJson(HostCommandResultJson(
            commandId, ok, ok ? "" : "failed to apply setting"));
      } else {
        const std::string windowId = payload->HasKey("windowId")
                                         ? payload->GetString("windowId").ToString()
                                         : "";
        const std::string tabId = payload->HasKey("tabId")
                                      ? payload->GetString("tabId").ToString()
                                      : "";
        BrowserWindow *target = !windowId.empty() ? FindWindow(windowId)
                                                    : FindWindowForTab(tabId);
        if (!target) {
          engine_.PushHostCommandResultJson(HostCommandResultJson(
              commandId, false, "target window or tab does not exist"));
        } else {
          target->ExecuteHostCommand(commandJson);
        }
      }
  }
  DispatchEngineEvents();
  if (processed == kMaxCommandsPerDrain ||
      std::chrono::steady_clock::now() - started >= kMaxDrainTime) {
    NotifyHostCommandReady();
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
  const std::string windowId = engineWindowId.empty() ? NextWindowId() : engineWindowId;
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
  (void)restoreState;
  return InvokeEngine(privateWindow ? "windows.createPrivate" : "windows.create");
}

bool BrowserAppController::RequestNewPrivateWindow() {
  return RequestNewWindow(true, nullptr);
}

bool BrowserAppController::CloseActiveWindow() {
  if (!activeWindow_) return false;
  auto params = CefDictionaryValue::Create();
  params->SetString("windowId", activeWindow_->WindowId());
  return InvokeEngine("windows.close", params);
}

bool BrowserAppController::ReopenClosedWindow() {
  return InvokeEngine("windows.reopenClosed");
}

bool BrowserAppController::RequestEngineCommand(
    const std::string &method, CefRefPtr<CefDictionaryValue> params) {
  return InvokeEngine(method, params);
}

bool BrowserAppController::RequestSettingChange(
    const std::string &tabId, const std::string &key, const std::string &value,
    bool reset, const std::string &returnUrl) {
  CEF_REQUIRE_UI_THREAD();
  if (tabId.empty() || key.empty()) {
    return false;
  }
  auto params = CefDictionaryValue::Create();
  params->SetString("key", key);
  if (!reset) {
    params->SetString("value", value);
  }
  auto request = CefDictionaryValue::Create();
  request->SetInt("version", 0);
  request->SetString("method", reset ? "settings.reset" : "settings.set");
  request->SetDictionary("params", params);
  auto requestValue = CefValue::Create();
  requestValue->SetDictionary(request);
  const std::string requestJson =
      CefWriteJSON(requestValue, JSON_WRITER_DEFAULT).ToString();

  return engine_.ProcessJsonAsync(
      requestJson, [this, tabId, returnUrl, key](std::string response) {
    auto parsed = CefParseJSON(response, JSON_PARSER_RFC);
    const bool ok = parsed && parsed->GetType() == VTYPE_DICTIONARY &&
                    (!parsed->GetDictionary()->HasKey("ok") ||
                     parsed->GetDictionary()->GetBool("ok"));
    CefPostTask(TID_UI, base::BindOnce(
        [](BrowserAppController *app, std::string tabId,
           std::string returnUrl, std::string key, bool ok) {
          if (!app || app != GetBrowserAppController()) {
            return;
          }
          if (!ok) {
            app->store_.AddLog("error", "Failed to update setting: " + key);
            return;
          }
          app->DispatchEngineEvents();
          if (BrowserWindow *window = app->FindWindowForTab(tabId)) {
            window->Navigate(tabId, returnUrl);
          }
        },
        this, tabId, returnUrl, key, ok));
  });
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
  // FrostEngine marks logical mutations dirty and persists a coalesced session
  // snapshot. Native deliberately does not write a second session copy.
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

BrowserWindow *BrowserAppController::FindWindow(const std::string &windowId) const {
  for (const auto &context : windows_) {
    if (context->window && context->window->WindowId() == windowId) {
      return context->window.get();
    }
  }
  return nullptr;
}

BrowserWindow *BrowserAppController::FindWindowForTab(const std::string &tabId) const {
  if (tabId.empty()) {
    return nullptr;
  }
  for (const auto &context : windows_) {
    if (context->window && context->window->Tabs().GetTab(tabId)) {
      return context->window.get();
    }
  }
  return nullptr;
}

bool BrowserAppController::InvokeEngine(const std::string &method,
                                        CefRefPtr<CefDictionaryValue> params) {
  auto request = CefDictionaryValue::Create();
  request->SetInt("version", 0);
  request->SetString("method", method);
  request->SetDictionary("params", params ? params : CefDictionaryValue::Create());
  auto value = CefValue::Create();
  value->SetDictionary(request);
  const std::string requestJson =
      CefWriteJSON(value, JSON_WRITER_DEFAULT).ToString();
  if (CefCurrentlyOn(TID_UI)) {
    return engine_.ProcessJsonAsync(requestJson, [this](std::string response) {
      auto parsed = CefParseJSON(response, JSON_PARSER_RFC);
      if (!parsed || parsed->GetType() != VTYPE_DICTIONARY ||
          (parsed->GetDictionary()->HasKey("ok") &&
           !parsed->GetDictionary()->GetBool("ok"))) {
        store_.AddLog("error", "FrostEngine request failed");
      }
    });
  }
  const std::string response = engine_.ProcessJson(requestJson);
  auto parsed = CefParseJSON(response, JSON_PARSER_RFC);
  return parsed && parsed->GetType() == VTYPE_DICTIONARY &&
         (!parsed->GetDictionary()->HasKey("ok") || parsed->GetDictionary()->GetBool("ok"));
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
    return app->RequestNewWindow(false, nullptr);
  }
  if (!window) {
    return false;
  }

  if (commandId == "tabs.activateNext") {
    return window->ActivateRelativeTab(true);
  }
  if (commandId == "tabs.activatePrevious") {
    return window->ActivateRelativeTab(false);
  }

  const std::string activeTabId = window->Tabs().GetActiveTabId();
  if (commandId == "tabs.create" || commandId == "tabs.reopenClosed" ||
      commandId == "tabs.home") {
    return app->RequestEngineCommand(commandId);
  }
  if (commandId == "tabs.close" || commandId == "tabs.duplicate" ||
      commandId == "tabs.closeOther" ||
      commandId == "tabs.closeToRight" ||
      commandId == "tabs.moveToNewWindow" || commandId == "tabs.reload" ||
      commandId == "tabs.stop" || commandId == "tabs.goBack" ||
      commandId == "tabs.goForward") {
    if (activeTabId.empty()) {
      return false;
    }
    auto params = CefDictionaryValue::Create();
    params->SetString("tabId", activeTabId);
    return app->RequestEngineCommand(commandId, params);
  }
  if (commandId == "tabs.pin" || commandId == "tabs.unpin") {
    if (activeTabId.empty()) {
      return false;
    }
    auto params = CefDictionaryValue::Create();
    params->SetString("tabId", activeTabId);
    params->SetBool("pinned", commandId == "tabs.pin");
    return app->RequestEngineCommand("tabs.pin", params);
  }

  std::string internalUrl;
  if (commandId == "app.openSettings") internalUrl = "fubuki://settings/";
  if (commandId == "app.openHistory") internalUrl = "fubuki://history/";
  if (commandId == "app.openDownloads") internalUrl = "fubuki://downloads/";
  if (commandId == "app.openBookmarks") internalUrl = "fubuki://bookmarks/";
  if (commandId == "app.openDebug") internalUrl = "fubuki://debug/";
  if (!internalUrl.empty()) {
    if (activeTabId.empty()) {
      auto params = CefDictionaryValue::Create();
      params->SetString("url", internalUrl);
      params->SetBool("active", true);
      return app->RequestEngineCommand("tabs.create", params);
    }
    auto params = CefDictionaryValue::Create();
    params->SetString("tabId", activeTabId);
    params->SetString("input", internalUrl);
    return app->RequestEngineCommand("tabs.navigate", params);
  }
  if (commandId == "app.toggleSidebar") {
    const std::string current = window->Store().GetSetting("sidebarVisible");
    auto params = CefDictionaryValue::Create();
    params->SetString("key", "sidebarVisible");
    params->SetString("value", current == "hide" ? "show" : "hide");
    return app->RequestEngineCommand("settings.set", params);
  }
  if (commandId == "bookmarks.addActive") {
    Tab *tab = window->Tabs().GetActiveTab();
    if (!tab || tab->url.empty()) {
      return false;
    }
    auto params = CefDictionaryValue::Create();
    params->SetString("title", tab->title.empty() ? tab->url : tab->title);
    params->SetString("url", tab->url);
    params->SetString("faviconUrl", tab->faviconUrl);
    return app->RequestEngineCommand("bookmarks.save", params);
  }

  // Page presentation commands (find, print, zoom and DevTools) intentionally
  // execute in the host because they do not own browser/session state.
  auto result = window->ExecuteCommand(commandId, CefDictionaryValue::Create());
  if (!result || result->GetType() == VTYPE_NULL) {
    return false;
  }
  return result->GetType() == VTYPE_BOOL ? result->GetBool() : true;
}

}  // namespace fubuki
