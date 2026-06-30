#pragma once

#include <memory>
#include <string>

#include "bridge/NativeBridge.h"
#include "browser/BrowserDataStore.h"
#include "browser/TabManager.h"
#include "commands/CommandRegistry.h"
#include "events/EventBus.h"
#include "include/cef_browser.h"
#include "include/cef_drag_handler.h"

#ifdef __OBJC__
@class NSWindow;
@class NSView;
#else
class NSWindow;
class NSView;
#endif

namespace fubuki {

class BrowserWindow {
 public:
  BrowserWindow(EventBus& eventBus, TabManager& tabManager);
  ~BrowserWindow();

  void Show();
  bool CreateTab(const std::string& input, bool active);
  bool ActivateTab(const std::string& tabId);
  bool CloseTab(const std::string& tabId);
  bool Navigate(const std::string& tabId, const std::string& input);
  bool Reload(const std::string& tabId);
  bool Stop(const std::string& tabId);
  bool GoBack(const std::string& tabId);
  bool GoForward(const std::string& tabId);
  bool FocusOmnibox();
  bool HandleShortcut(bool commandDown, bool altDown, int keyCode, char character);
  bool OpenDevTools();
  bool AddActiveBookmark();
  bool SaveBookmark(const std::string& title, const std::string& url, const std::string& faviconUrl);
  bool RemoveBookmark(const std::string& url);
  bool RemoveHistory(const std::string& url);
  bool RemoveDownload(const std::string& url, const std::string& path);
  bool ClearBrowsingData(const std::string& target);
  bool SetSetting(const std::string& key, const std::string& value);
  bool SetUiOverlayActive(bool active);
  bool HandleSettingsUrl(const std::string& tabId, const std::string& url);
  bool HandleNewTabSearchUrl(const std::string& tabId, const std::string& url);
  std::string DownloadPathFor(const std::string& suggestedName) const;

  void SetUiBrowser(CefRefPtr<CefBrowser> browser);
  void OnTabBrowserCreated(const std::string& tabId, CefRefPtr<CefBrowser> browser);
  void OnTabTitle(const std::string& tabId, const std::string& title);
  void OnTabUrl(const std::string& tabId, const std::string& url);
  void OnTabFavicon(const std::string& tabId, const std::string& faviconUrl);
  void OnTabLoadingState(const std::string& tabId, bool isLoading, bool canGoBack, bool canGoForward);
  void OnNavigationStarted(const std::string& tabId);
  void OnNavigationFinished(const std::string& tabId);
  void OnNavigationFailed(const std::string& tabId, const std::string& message);
  void OnDownloadStarted(const std::string& url, const std::string& path);
  void OnDownloadUpdated(const std::string& url, const std::string& path, const std::string& state, int percent);
  void OnUiDraggableRegionsChanged(const std::vector<CefDraggableRegion>& regions);

  NativeBridge* Bridge() { return bridge_.get(); }
  CommandRegistry& Commands() { return commands_; }
  TabManager& Tabs() { return tabManager_; }
  const TabManager& Tabs() const { return tabManager_; }
  BrowserDataStore& Store() { return *dataStore_; }
  const BrowserDataStore& Store() const { return *dataStore_; }
  CefRefPtr<CefBrowser> UiBrowser() const { return uiBrowser_; }

 private:
  void CreateNativeWindow();
  void CreateUiBrowser();
  void CreateTabBrowser(const Tab& tab);
  void RegisterCommands();
  void WireEvents();
  void ResizeViews();
  void UpdateContentFrame();
  void UpdateTabPatch(const std::string& tabId,
                      const std::string& title,
                      const std::string& url,
                      bool isLoading,
                      bool canGoBack,
                      bool canGoForward);
  void SetActiveContentView();
  CefWindowHandle ContentParentHandle() const;
  CefWindowHandle UiParentHandle() const;

  EventBus& eventBus_;
  TabManager& tabManager_;
  CommandRegistry commands_;
  std::unique_ptr<NativeBridge> bridge_;
  std::unique_ptr<BrowserDataStore> dataStore_;
  CefRefPtr<CefBrowser> uiBrowser_;
  NSWindow* window_ = nullptr;
  NSView* uiHostView_ = nullptr;
  NSView* contentHostView_ = nullptr;
  NSView* dragRegionView_ = nullptr;
};

}  // namespace fubuki
