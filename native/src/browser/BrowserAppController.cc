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

bool EngineRequestSucceeded(FrostBridge &engine, const std::string &method,
                            CefRefPtr<CefDictionaryValue> params = nullptr) {
  auto request = CefDictionaryValue::Create();
  request->SetInt("version", 0);
  request->SetString("method", method);
  if (params) request->SetDictionary("params", params);
  auto value = CefValue::Create();
  value->SetDictionary(request);
  auto response = CefParseJSON(
      engine.ProcessJson(CefWriteJSON(value, JSON_WRITER_DEFAULT).ToString()),
      JSON_PARSER_RFC);
  return response && response->GetType() == VTYPE_DICTIONARY &&
         response->GetDictionary()->HasKey("ok") &&
         response->GetDictionary()->GetBool("ok");
}

std::string WindowHostEventJson(const std::string &event,
                                const std::string &windowId) {
  auto payload = CefDictionaryValue::Create();
  payload->SetString("windowId", windowId);
  auto envelope = CefDictionaryValue::Create();
  envelope->SetInt("version", 0);
  envelope->SetString("event", event);
  envelope->SetDictionary("payload", payload);
  auto value = CefValue::Create();
  value->SetDictionary(envelope);
  return CefWriteJSON(value, JSON_WRITER_DEFAULT).ToString();
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
  // FFI bootstraps the initial Rust-owned window as a HostCommand.  Native
  // only starts pumping; it never invents a startup id or window state.
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
  // operationId is the protocol name.  Rust accepts commandId while older
  // engines are present during upgrades, but native always emits the new form.
  root->SetString("operationId", commandId);
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
  CEF_REQUIRE_UI_THREAD();
  DispatchHostCommandsFor(engine_);

  // Do not iterate the vector by reference: processing a command can create
  // another private runtime, which may reallocate privateEngines_.
  std::vector<FrostBridge *> privateEngines;
  privateEngines.reserve(privateEngines_.size());
  for (const auto &runtime : privateEngines_) {
    if (runtime) privateEngines.push_back(runtime.get());
  }
  for (FrostBridge *runtime : privateEngines) {
    DispatchHostCommandsFor(*runtime);
  }
}

void BrowserAppController::DispatchHostCommandsFor(FrostBridge &engine) {
  CEF_REQUIRE_UI_THREAD();
  std::string commandJson;
  while (engine.PollHostCommandJson(commandJson)) {
    CefRefPtr<CefValue> value = CefParseJSON(commandJson, JSON_PARSER_RFC);
    if (!value || value->GetType() != VTYPE_DICTIONARY) {
      LOG(ERROR) << "[FrostRuntime] rejected malformed HostCommand";
      continue;
    }
    CefRefPtr<CefDictionaryValue> envelope = value->GetDictionary();
    const std::string command = envelope->HasKey("command")
                                    ? envelope->GetString("command").ToString()
                                    : "";
    const std::string operationId = envelope->HasKey("operationId")
                                        ? envelope->GetString("operationId").ToString()
                                        : (envelope->HasKey("id")
                                               ? envelope->GetString("id").ToString()
                                               : "");
    const CefRefPtr<CefDictionaryValue> payload =
        envelope->HasKey("payload") && envelope->GetType("payload") == VTYPE_DICTIONARY
            ? envelope->GetDictionary("payload")
            : CefDictionaryValue::Create();
    if (operationId.empty() || command.empty()) {
      LOG(ERROR) << "[FrostRuntime] HostCommand has no operationId or command";
      if (!operationId.empty()) {
        PushHostCommandResult(engine, operationId, false, "malformed host command");
      }
      continue;
    }

    bool ok = false;
    bool deferResult = false;
    std::string error;
    if (command == "runtime.createPrivate") {
      // The persistent engine never owns private WindowState. It asks the
      // runtime to create a separate in-memory engine and this operation is
      // complete only after that engine's private bootstrap host window was
      // created successfully.
      ok = RequestNewPrivateWindow();
      if (!ok) error = "failed to create isolated private runtime";
    } else if (command == "window.create") {
      const std::string windowId = payload->GetString("windowId").ToString();
      const bool isPrivate = payload->HasKey("isPrivate") && payload->GetBool("isPrivate");
      // An in-memory FrostEngine is a private runtime even for its bootstrap
      // window. Never permit a private host window to be created by the
      // profile-backed runtime, because that would put its state in the
      // regular profile before the host has a chance to isolate it.
      const bool runtimeIsPrivate = engine.IsEphemeral();
      if (isPrivate && !runtimeIsPrivate) {
        error = "private windows require an isolated in-memory FrostEngine";
      } else {
        ok = !windowId.empty() &&
             NewWindowForEngine(engine, runtimeIsPrivate || isPrivate, nullptr,
                                windowId) != nullptr;
        if (!ok) error = "failed to create window";
      }
    } else if (command == "window.close") {
      const std::string windowId = payload->GetString("windowId").ToString();
      BrowserWindow *window = FindWindowById(windowId, &engine);
      if (deferredWindowCloses_.contains(windowId)) {
        error = "window is already closing";
      } else if (window) {
        // Register before performClose: AppKit may synchronously invoke the
        // delegate during a close request.
        deferredWindowCloses_.emplace(
            windowId, DeferredWindowClose{&engine, operationId});
        for (const auto &context : windows_) {
          if (context->window.get() == window) {
            context->closing = true;
            break;
          }
        }
        ok = window->CloseWindow();
        if (ok) {
          deferResult = true;
        } else {
          deferredWindowCloses_.erase(windowId);
          for (const auto &context : windows_) {
            if (context->window.get() == window) {
              context->closing = false;
              break;
            }
          }
        }
      }
      if (!ok) error = "unknown window or close failed";
    } else {
      const std::string windowId = payload->GetString("windowId").ToString();
      const std::string tabId = payload->GetString("tabId").ToString();
      BrowserWindow *window = !windowId.empty() ? FindWindowById(windowId, &engine)
                              : !tabId.empty() ? FindWindowForTab(tabId, &engine)
                                               : ActiveWindow();
      if (window && FindEngineForWindow(window) != &engine) {
        window = nullptr;
      }
      if (!window) {
        error = "no host window owns command target";
      } else {
        const bool isPageCreate = command == "page.create";
        const bool isPageClose = command == "page.close";
        if (isPageCreate || isPageClose) {
          if (tabId.empty()) {
            error = "malformed page lifecycle command";
          } else if ((isPageCreate ? deferredPageCreates_ : deferredPageCloses_)
                         .contains(tabId)) {
            error = "duplicate pending page lifecycle command";
          } else {
            // Register before calling CEF: CloseBrowser can synchronously
            // reach OnBeforeClose on some CEF paths.
            (isPageCreate ? deferredPageCreates_ : deferredPageCloses_)
                .emplace(tabId, DeferredWindowClose{&engine, operationId});
            deferResult = true;
          }
        }
        if (error.empty()) ok = window->ExecuteHostCommand(commandJson, &error);
        if (!ok && (isPageCreate || isPageClose)) {
          (isPageCreate ? deferredPageCreates_ : deferredPageCloses_).erase(tabId);
          deferResult = false;
        }
      }
    }
    if (!deferResult) {
      PushHostCommandResult(engine, operationId, ok,
                            error.empty() && !ok ? "host command failed" : error);
    }
  }

  // Frost events are emitted after core state has accepted a host result. Do
  // not broadcast tab/window events to every UI: resolve their owning host
  // first so multiple windows cannot render each other's state changes.
  std::string eventJson;
  while (engine.PollEventJson(eventJson)) {
    CefRefPtr<CefValue> value = CefParseJSON(eventJson, JSON_PARSER_RFC);
    if (!value || value->GetType() != VTYPE_DICTIONARY) {
      LOG(ERROR) << "[FrostRuntime] rejected malformed Frost event";
      continue;
    }
    CefRefPtr<CefDictionaryValue> envelope = value->GetDictionary();
    const std::string event = envelope->GetString("event").ToString();
    CefRefPtr<CefDictionaryValue> payload =
        envelope->HasKey("payload") && envelope->GetType("payload") == VTYPE_DICTIONARY
            ? envelope->GetDictionary("payload")
            : CefDictionaryValue::Create();

    if (event == "host.operationCompleted") {
      const std::string operationId =
          payload->HasKey("operationId")
              ? payload->GetString("operationId").ToString()
              : "";
      const std::string status =
          payload->HasKey("status") ? payload->GetString("status").ToString()
                                    : "";
      const std::string error =
          payload->HasKey("error") ? payload->GetString("error").ToString()
                                   : "Host operation failed";
      const bool succeeded = status == "succeeded";
      const int errorCode = status == "timedOut" ? 504 : 500;
      bool completed = false;
      if (!operationId.empty()) {
        // The originating renderer can be in any window of this runtime.
        // NativeBridge owns the CEF callback, so ask every scoped bridge and
        // let exactly one matching operation ID consume it.
        for (const auto &context : windows_) {
          if (context->engine == &engine && context->window &&
              context->window->Bridge()) {
            completed = context->window->Bridge()->CompletePendingOperation(
                            operationId, succeeded, {}, error, errorCode) ||
                        completed;
          }
        }
      }
      if (!completed && !operationId.empty()) {
        LOG(INFO) << "[FrostRuntime] terminal host operation has no renderer "
                     "callback: "
                  << operationId;
      }
      // A completion is an acknowledgement for a particular bridge request,
      // not a state update for an arbitrary window. The domain event emitted
      // immediately before it carries the state change to the correct UI.
      continue;
    }
    const std::string windowId = payload->HasKey("windowId")
                                     ? payload->GetString("windowId").ToString()
                                     : (event == "window.created"
                                            ? payload->GetString("id").ToString()
                                            : "");
    const std::string tabId = payload->GetString("tabId").ToString();
    BrowserWindow *target = !windowId.empty() ? FindWindowById(windowId, &engine)
                            : !tabId.empty() ? FindWindowForTab(tabId, &engine)
                                             : nullptr;

    if (event == "tab.activated" && target && !tabId.empty()) {
      target->ActivateTab(tabId);
    }
    if (target && target->Bridge()) {
      target->Bridge()->EmitToUi(event, payload);
      target->Bridge()->EmitToUi("app.stateChanged", CefDictionaryValue::Create());
    } else if (event == "setting.changed" || event == "bookmark.changed" ||
               event == "history.changed" || event == "download.changed") {
      // Persistent data events are visible to every normal-profile window.
      // An ephemeral engine has no cross-window sharing, so only deliver to
      // its own runtime (normally exactly one private window).
      for (const auto &context : windows_) {
        if (context->engine == &engine && context->window &&
            context->window->Bridge()) {
          context->window->Bridge()->EmitToUi(event, payload);
          context->window->Bridge()->EmitToUi("app.stateChanged",
                                              CefDictionaryValue::Create());
        }
      }
    } else if (!target && event != "host.synced") {
      LOG(WARNING) << "[FrostRuntime] no UI target for Frost event " << event;
    }
  }
}

BrowserWindow *
BrowserAppController::NewWindow(bool privateWindow,
                                CefRefPtr<CefDictionaryValue> restoreState,
                                const std::string &engineWindowId) {
  return NewWindowForEngine(engine_, privateWindow, restoreState, engineWindowId);
}

BrowserWindow *BrowserAppController::NewWindowForEngine(
    FrostBridge &engine, bool privateWindow,
    CefRefPtr<CefDictionaryValue> restoreState,
    const std::string &engineWindowId) {
  CEF_REQUIRE_UI_THREAD();
  auto context = std::make_unique<WindowContext>();
  context->tabManager = std::make_unique<TabManager>(eventBus_);
  BrowserWindow *raw = nullptr;
  if (engineWindowId.empty()) {
    LOG(ERROR) << "[FrostRuntime] refused host window without Rust window id";
    return nullptr;
  }
  const std::string &windowId = engineWindowId;
  if (FindWindowById(windowId, &engine)) {
    LOG(ERROR) << "[FrostRuntime] refused duplicate window id " << windowId;
    return nullptr;
  }
  context->window = std::make_unique<BrowserWindow>(*this, *context->tabManager,
                                                    windowId, privateWindow);
  context->engine = &engine;
  raw = context->window.get();
  windows_.push_back(std::move(context));
  activeWindow_ = raw;
  raw->Show(restoreState);
  // FrostRuntime publishes window.created only after the matching HostCommand
  // result has been accepted; native must not send an optimistic duplicate.
  return raw;
}

bool BrowserAppController::NewPrivateWindow() {
  return RequestNewPrivateWindow();
}

bool BrowserAppController::RequestNewWindow(
    bool privateWindow, CefRefPtr<CefDictionaryValue> restoreState) {
  return RequestNewWindowFor(nullptr, privateWindow, restoreState);
}

bool BrowserAppController::RequestNewWindowFor(
    const BrowserWindow *owner, bool privateWindow,
    CefRefPtr<CefDictionaryValue> restoreState) {
  if (restoreState) {
    LOG(WARNING) << "[FrostRuntime] restoreState is not supported outside FrostEngine";
    return false;
  }
  // A private window always receives a fresh in-memory engine. This is never
  // expressed as windows.createPrivate to the persistent runtime.
  if (privateWindow || (owner && owner->IsPrivate())) {
    return RequestNewPrivateWindow();
  }
  return EngineRequestSucceeded(EngineForWindow(owner), "windows.create");
}

bool BrowserAppController::RequestNewPrivateWindow() {
  CEF_REQUIRE_UI_THREAD();
  FrostBridge *privateEngine = CreatePrivateEngine();
  if (!privateEngine) {
    return false;
  }

  // frost_engine_new_private() bootstraps one Rust-owned private window.
  // Drain it on the UI thread so the host exists before this method reports
  // acceptance. The isolated engine can never open the profile SQLite store.
  DispatchHostCommandsFor(*privateEngine);
  return std::any_of(windows_.begin(), windows_.end(),
                     [privateEngine](const auto &context) {
                       return context->engine == privateEngine && context->window;
                     });
}

bool BrowserAppController::CloseActiveWindow() {
  if (!activeWindow_) {
    return false;
  }
  auto params = CefDictionaryValue::Create();
  params->SetString("windowId", activeWindow_->WindowId());
  return EngineRequestSucceeded(EngineForWindow(activeWindow_), "windows.close", params);
}

bool BrowserAppController::ReopenClosedWindow() {
  return EngineRequestSucceeded(EngineForWindow(activeWindow_), "windows.reopenClosed");
}

void BrowserAppController::NotifyWindowFocused(BrowserWindow *window) {
  activeWindow_ = window;
  if (window) {
    FrostBridge *engine = FindEngineForWindow(window);
    if (!engine || !engine->PushHostEventJson(
            WindowHostEventJson("window.focused", window->WindowId()))) {
      LOG(ERROR) << "[FrostRuntime] failed to publish window.focused";
    }
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
  FrostBridge *engine = FindEngineForWindow(window);
  if (!engine || !engine->PushHostEventJson(
                     WindowHostEventJson("window.closed", windowId))) {
    LOG(ERROR) << "[FrostRuntime] failed to publish window.closed";
  }
  auto it = std::find_if(windows_.begin(), windows_.end(),
                         [&](const std::unique_ptr<WindowContext> &context) {
                           return context->window.get() == window;
                         });
  if (it != windows_.end()) {
    // FubukiClient retains a raw BrowserWindow pointer until CEF delivers its
    // final close callbacks. Keep the context alive for the controller's
    // lifetime, but exclude it from every target lookup below.
    (*it)->closing = true;
  }
  activeWindow_ = nullptr;
  for (auto candidate = windows_.rbegin(); candidate != windows_.rend(); ++candidate) {
    if ((*candidate)->window && !(*candidate)->closing) {
      activeWindow_ = (*candidate)->window.get();
      break;
    }
  }
  if (auto pending = deferredWindowCloses_.find(windowId);
      pending != deferredWindowCloses_.end()) {
    if (!PushHostCommandResult(*pending->second.engine,
                               pending->second.operationId, true, "")) {
      LOG(ERROR) << "[FrostRuntime] failed to acknowledge closed NSWindow";
    }
    deferredWindowCloses_.erase(pending);
  }
  eventBus_.Publish(
      {EventType::WindowClosed, "window.closed", {}, windowId, "", ""});
}

void BrowserAppController::NotifyPageCreated(BrowserWindow *window,
                                             const std::string &tabId) {
  CEF_REQUIRE_UI_THREAD();
  if (!window || tabId.empty()) return;
  auto pending = deferredPageCreates_.find(tabId);
  if (pending == deferredPageCreates_.end()) return;
  FrostBridge *engine = FindEngineForWindow(window);
  if (!engine || engine != pending->second.engine) {
    LOG(ERROR) << "[FrostRuntime] page creation completed for wrong runtime";
    return;
  }
  auto payload = CefDictionaryValue::Create();
  payload->SetString("tabId", tabId);
  payload->SetString("windowId", window->WindowId());
  if (Tab *tab = window->Tabs().GetTab(tabId)) {
    payload->SetString("url", tab->url);
  }
  auto envelope = CefDictionaryValue::Create();
  envelope->SetInt("version", 0);
  envelope->SetString("event", "page.created");
  envelope->SetDictionary("payload", payload);
  auto value = CefValue::Create();
  value->SetDictionary(envelope);
  if (!engine->PushHostEventJson(
          CefWriteJSON(value, JSON_WRITER_DEFAULT).ToString()) ||
      !PushHostCommandResult(*engine, pending->second.operationId, true, "")) {
    LOG(ERROR) << "[FrostRuntime] failed to finalize CEF page creation";
  }
  deferredPageCreates_.erase(pending);
}

void BrowserAppController::NotifyPageClosed(BrowserWindow *window,
                                            const std::string &tabId) {
  CEF_REQUIRE_UI_THREAD();
  if (!window || tabId.empty()) return;
  auto pending = deferredPageCloses_.find(tabId);
  if (pending == deferredPageCloses_.end()) return;
  FrostBridge *engine = FindEngineForWindow(window);
  if (!engine || engine != pending->second.engine) {
    LOG(ERROR) << "[FrostRuntime] page close completed for wrong runtime";
    return;
  }
  auto payload = CefDictionaryValue::Create();
  payload->SetString("tabId", tabId);
  auto envelope = CefDictionaryValue::Create();
  envelope->SetInt("version", 0);
  envelope->SetString("event", "page.closed");
  envelope->SetDictionary("payload", payload);
  auto value = CefValue::Create();
  value->SetDictionary(envelope);
  if (!engine->PushHostEventJson(
          CefWriteJSON(value, JSON_WRITER_DEFAULT).ToString()) ||
      !PushHostCommandResult(*engine, pending->second.operationId, true, "")) {
    LOG(ERROR) << "[FrostRuntime] failed to finalize CEF page close";
  }
  deferredPageCloses_.erase(pending);
}

void BrowserAppController::PersistSession() {
  CEF_REQUIRE_UI_THREAD();
  // Session persistence is performed by FrostEngine. Native deliberately has
  // no serialized session copy to avoid a second source of truth.
}

BrowserWindow *BrowserAppController::ActiveWindow() const {
  return activeWindow_;
}

std::vector<BrowserWindow *> BrowserAppController::Windows() const {
  std::vector<BrowserWindow *> result;
  for (const auto &context : windows_) {
    if (context->window && !context->closing) {
      result.push_back(context->window.get());
    }
  }
  return result;
}

FrostBridge &BrowserAppController::EngineForWindow(const BrowserWindow *window) {
  return *FindEngineForWindow(window);
}

const FrostBridge &BrowserAppController::EngineForWindow(
    const BrowserWindow *window) const {
  return *FindEngineForWindow(window);
}

FrostBridge *BrowserAppController::EngineForWindowId(const std::string &windowId) {
  BrowserWindow *window = FindWindowById(windowId);
  return window ? FindEngineForWindow(window) : nullptr;
}

const FrostBridge *BrowserAppController::EngineForWindowId(
    const std::string &windowId) const {
  BrowserWindow *window = FindWindowById(windowId);
  return window ? FindEngineForWindow(window) : nullptr;
}

FrostBridge *BrowserAppController::FindEngineForWindow(const BrowserWindow *window) {
  return const_cast<FrostBridge *>(
      static_cast<const BrowserAppController *>(this)->FindEngineForWindow(window));
}

const FrostBridge *BrowserAppController::FindEngineForWindow(
    const BrowserWindow *window) const {
  if (!window) return &engine_;
  for (const auto &context : windows_) {
    if (context->window.get() == window) {
      return context->engine ? context->engine : &engine_;
    }
  }
  LOG(ERROR) << "[FrostRuntime] unknown BrowserWindow engine lookup";
  return &engine_;
}

FrostBridge *BrowserAppController::CreatePrivateEngine() {
  auto runtime = std::make_unique<FrostBridge>();
  if (!runtime->IsAvailable()) {
    LOG(ERROR) << "[FrostRuntime] could not initialize private in-memory engine";
    return nullptr;
  }
  FrostBridge *raw = runtime.get();
  privateEngines_.push_back(std::move(runtime));
  return raw;
}

BrowserWindow *BrowserAppController::FindWindowById(const std::string &windowId,
                                                     const FrostBridge *engine) const {
  for (const auto &context : windows_) {
    if (context->window && !context->closing && (!engine || context->engine == engine) &&
        context->window->WindowId() == windowId) {
      return context->window.get();
    }
  }
  return nullptr;
}

BrowserWindow *BrowserAppController::FindWindowForTab(const std::string &tabId,
                                                       const FrostBridge *engine) const {
  for (const auto &context : windows_) {
    if (context->window && !context->closing && (!engine || context->engine == engine) &&
        context->window->Tabs().GetTab(tabId)) {
      return context->window.get();
    }
  }
  return nullptr;
}

bool BrowserAppController::PushHostCommandResult(FrostBridge &engine,
                                                  const std::string &operationId,
                                                  bool ok,
                                                  const std::string &error) {
  if (operationId.empty()) {
    return false;
  }
  return engine.PushHostCommandResultJson(
      HostCommandResultJson(operationId, ok, error));
}

BrowserAppController *GetBrowserAppController() {
  return gController;
}

void SetBrowserAppController(BrowserAppController *controller) {
  gController = controller;
}

bool DispatchBrowserMenuCommand(const std::string &commandId) {
  BrowserAppController *app = GetBrowserAppController();
  if (!app) {
    return false;
  }
  auto params = CefDictionaryValue::Create();
  params->SetString("id", commandId);
  // Native menus are input surfaces only. Command interpretation and state
  // changes are shared with the UI via FrostEngine's commands.execute path.
  return EngineRequestSucceeded(app->EngineForWindow(app->ActiveWindow()),
                                "commands.execute", params);
}

}  // namespace fubuki
