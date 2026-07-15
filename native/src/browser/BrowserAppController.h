#pragma once

#include <filesystem>
#include <memory>
#include <string>
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

  // The engine bridge is owned by the controller and shared with windows so
  // that browser-data mutations go through the protocol layer.
  FrostBridge &Engine() { return engine_; }
  const FrostBridge &Engine() const { return engine_; }

  void Start();
  BrowserWindow *
  NewWindow(bool privateWindow = false,
            CefRefPtr<CefDictionaryValue> restoreState = nullptr,
            const std::string &engineWindowId = "");
  bool NewPrivateWindow();
  bool RequestNewWindow(bool privateWindow = false,
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
  void PersistSession();

  BrowserWindow *ActiveWindow() const;
  BrowserWindow *FindWindow(const std::string &windowId) const;
  BrowserWindow *FindWindowForTab(const std::string &tabId) const;
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
  };

  std::string NextWindowId();
  CefRefPtr<CefListValue> RestoredWindows() const;
  void FinalizeWindowClosed(BrowserWindow *window);

  std::filesystem::path profilePath_;
  FrostBridge engine_;
  FrostStore store_;
  EventBus eventBus_;
  std::vector<std::unique_ptr<WindowContext>> windows_;
  std::vector<CefRefPtr<CefDictionaryValue>> closedWindows_;
  BrowserWindow *activeWindow_ = nullptr;
  int nextWindowId_ = 1;
  bool restoring_ = false;
};

BrowserAppController *GetBrowserAppController();
void SetBrowserAppController(BrowserAppController *controller);
bool DispatchBrowserMenuCommand(const std::string &commandId);

}  // namespace fubuki
