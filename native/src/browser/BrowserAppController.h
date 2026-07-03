#pragma once

#include <filesystem>
#include <memory>
#include <string>
#include <vector>

#include "browser/BrowserDataStore.h"
#include "browser/TabManager.h"
#include "events/EventBus.h"
#include "include/cef_values.h"

namespace fubuki {

class BrowserWindow;

class BrowserAppController {
 public:
  explicit BrowserAppController(std::filesystem::path profilePath);
  ~BrowserAppController();

  void Start();
  BrowserWindow* NewWindow(bool privateWindow = false, CefRefPtr<CefDictionaryValue> restoreState = nullptr);
  bool NewPrivateWindow();
  bool RequestNewWindow(bool privateWindow = false, CefRefPtr<CefDictionaryValue> restoreState = nullptr);
  bool RequestNewPrivateWindow();
  bool CloseActiveWindow();
  bool ReopenClosedWindow();
  void NotifyWindowFocused(BrowserWindow* window);
  void NotifyWindowClosed(BrowserWindow* window);
  void PersistSession();

  BrowserWindow* ActiveWindow() const;
  std::vector<BrowserWindow*> Windows() const;
  BrowserDataStore& Store() { return store_; }
  const BrowserDataStore& Store() const { return store_; }
  EventBus& Events() { return eventBus_; }
  const EventBus& Events() const { return eventBus_; }

 private:
  struct WindowContext {
    std::unique_ptr<TabManager> tabManager;
    std::unique_ptr<BrowserWindow> window;
  };

  std::string NextWindowId();
  CefRefPtr<CefListValue> RestoredWindows() const;

  std::filesystem::path profilePath_;
  BrowserDataStore store_;
  EventBus eventBus_;
  std::vector<std::unique_ptr<WindowContext>> windows_;
  std::vector<CefRefPtr<CefDictionaryValue>> closedWindows_;
  BrowserWindow* activeWindow_ = nullptr;
  int nextWindowId_ = 1;
  bool restoring_ = false;
};

BrowserAppController* GetBrowserAppController();
void SetBrowserAppController(BrowserAppController* controller);
bool DispatchBrowserMenuCommand(const std::string& commandId);

}  // namespace fubuki
