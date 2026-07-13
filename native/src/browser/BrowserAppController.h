#pragma once

#include <filesystem>
#include <memory>
#include <string>
#include <unordered_map>
#include <vector>

#include "browser/FrostStore.h"
#include "browser/TabManager.h"
#include "bridge/FrostBridge.h"
#include "events/EventBus.h"
#include "include/cef_values.h"

namespace fubuki {

class BrowserWindow;

class BrowserAppController {
public:
  explicit BrowserAppController(std::filesystem::path profilePath);
  ~BrowserAppController();

  // The profile-backed engine is the only persistent runtime. It remains for
  // process-wide callers that do not have a window target; UI and host code
  // must use EngineForWindow so private windows remain in their own runtime.
  FrostBridge &Engine() { return engine_; }
  const FrostBridge &Engine() const { return engine_; }
  FrostBridge &EngineForWindow(const BrowserWindow *window);
  const FrostBridge &EngineForWindow(const BrowserWindow *window) const;
  FrostBridge *EngineForWindowId(const std::string &windowId);
  const FrostBridge *EngineForWindowId(const std::string &windowId) const;

  void Start();
  BrowserWindow *
  NewWindow(bool privateWindow = false,
            CefRefPtr<CefDictionaryValue> restoreState = nullptr,
            const std::string &engineWindowId = "");
  bool NewPrivateWindow();
  bool RequestNewWindow(bool privateWindow = false,
                        CefRefPtr<CefDictionaryValue> restoreState = nullptr);
  bool RequestNewWindowFor(const BrowserWindow *owner, bool privateWindow,
                           CefRefPtr<CefDictionaryValue> restoreState = nullptr);
  bool RequestNewPrivateWindow();
  bool CloseActiveWindow();
  bool ReopenClosedWindow();
  // Polls FrostEngine HostCommands and executes them. Window lifecycle
  // commands are handled here; page/overlay commands are delegated to the
  // owning BrowserWindow.
  void DispatchHostCommands();
  // Starts the self-rescheduling host command poller on the CEF UI thread.
  void StartHostCommandPoller();
  void NotifyWindowFocused(BrowserWindow *window);
  void NotifyWindowClosed(BrowserWindow *window);
  void NotifyPageCreated(BrowserWindow *window, const std::string &tabId);
  void NotifyPageClosed(BrowserWindow *window, const std::string &tabId);
  void PersistSession();

  BrowserWindow *ActiveWindow() const;
  std::vector<BrowserWindow *> Windows() const;
  FrostStore &Store() {
    return store_;
  }
  const FrostStore &Store() const {
    return store_;
  }
  EventBus &Events() {
    return eventBus_;
  }
  const EventBus &Events() const {
    return eventBus_;
  }

private:
  struct WindowContext {
    std::unique_ptr<TabManager> tabManager;
    std::unique_ptr<BrowserWindow> window;
    // Non-owning: persistent engine_ or an entry in privateEngines_. Keeping
    // the engine separate from the BrowserWindow makes its lifetime outlast
    // CEF callbacks issued while a window is closing.
    FrostBridge *engine = nullptr;
    bool closing = false;
  };

  struct DeferredWindowClose {
    FrostBridge *engine = nullptr;
    std::string operationId;
  };

  BrowserWindow *NewWindowForEngine(FrostBridge &engine, bool privateWindow,
                                    CefRefPtr<CefDictionaryValue> restoreState,
                                    const std::string &engineWindowId);
  BrowserWindow *FindWindowById(const std::string &windowId,
                                const FrostBridge *engine = nullptr) const;
  BrowserWindow *FindWindowForTab(const std::string &tabId,
                                  const FrostBridge *engine = nullptr) const;
  FrostBridge *FindEngineForWindow(const BrowserWindow *window);
  const FrostBridge *FindEngineForWindow(const BrowserWindow *window) const;
  void DispatchHostCommandsFor(FrostBridge &engine);
  bool PushHostCommandResult(FrostBridge &engine,
                             const std::string &operationId, bool ok,
                             const std::string &error);
  FrostBridge *CreatePrivateEngine();

  std::filesystem::path profilePath_;
  FrostBridge engine_;
  // Private runtimes are intentionally retained until controller teardown.
  // This prevents a late CEF callback from dereferencing a freed engine while
  // still guaranteeing that no private state reaches the profile store.
  std::vector<std::unique_ptr<FrostBridge>> privateEngines_;
  FrostStore store_;
  EventBus eventBus_;
  std::vector<std::unique_ptr<WindowContext>> windows_;
  // An NSWindow close is asynchronous. Keep its operation pending until the
  // delegate has observed the real close, rather than acknowledging merely
  // because performClose accepted the request.
  std::unordered_map<std::string, DeferredWindowClose> deferredWindowCloses_;
  std::unordered_map<std::string, DeferredWindowClose> deferredPageCreates_;
  std::unordered_map<std::string, DeferredWindowClose> deferredPageCloses_;
  BrowserWindow *activeWindow_ = nullptr;
};

BrowserAppController *GetBrowserAppController();
void SetBrowserAppController(BrowserAppController *controller);
bool DispatchBrowserMenuCommand(const std::string &commandId);

}  // namespace fubuki
