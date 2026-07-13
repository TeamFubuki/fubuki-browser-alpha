#pragma once

#include <memory>
#include <string>
#include <utility>
#include <vector>

#include "bridge/NativeBridge.h"
#include "browser/FrostStore.h"
#include "browser/TabManager.h"
#include "commands/CommandRegistry.h"
#include "events/EventBus.h"
#include "include/cef_browser.h"
#include "include/cef_drag_handler.h"
#include "include/cef_request_context.h"

#ifdef __OBJC__
@class NSWindow;
@class NSView;
#else
class NSWindow;
class NSView;
#endif

namespace fubuki {

class BrowserAppController;

class BrowserWindow {
 public:
  BrowserWindow(BrowserAppController& app, TabManager& tabManager, std::string windowId,
                bool privateWindow);
  ~BrowserWindow();

  void Show(CefRefPtr<CefDictionaryValue> restoreState = nullptr);
  bool CloseWindow();
  bool CreateTab(const std::string &input, bool active);
  // Creates a tab whose id is owned by an external authority (FrostEngine).
  bool CreateTabWithId(const std::string &input, const std::string &tabId,
                       bool active);
  std::string CreatePendingPopupTab(const std::string &url, bool active);
  bool ActivateTab(const std::string &tabId);
  bool CloseTab(const std::string &tabId);
  bool PinTab(const std::string &tabId, bool pinned);
  bool DuplicateTab(const std::string &tabId);
  bool ReopenClosedTab();
  bool CloseOtherTabs(const std::string& tabId);
  bool CloseTabsToRight(const std::string& tabId);
  bool MoveTab(const std::string& tabId, int toIndex);
  bool MoveTabToNewWindow(const std::string& tabId);
  bool Navigate(const std::string& tabId, const std::string& input);
  bool Reload(const std::string& tabId);
  bool Stop(const std::string& tabId);
  bool GoBack(const std::string& tabId);
  bool GoForward(const std::string& tabId);
  bool GoHome();
  bool FindInPage(const std::string& query, bool forward);
  bool StopFinding(bool clearSelection);
  bool ZoomIn();
  bool ZoomOut();
  bool ResetZoom();
  bool PrintPage();
  bool ViewSource();
  bool FocusOmnibox();
  CefRefPtr<CefValue> ExecuteCommand(const std::string& commandId,
                                     CefRefPtr<CefDictionaryValue> args);
  bool HandleShortcut(bool commandDown, bool altDown, int keyCode, char character);
  bool OpenDevTools();
  bool AddActiveBookmark();
  bool SaveBookmark(const std::string& title, const std::string& url,
                    const std::string& faviconUrl);
  bool RemoveBookmark(const std::string& url);
  bool RemoveHistory(const std::string& url);
  bool RemoveDownload(const std::string& url, const std::string& path);
  bool OpenDownloadedFile(const std::string& path);
  bool RevealDownloadedFile(const std::string& path);
  bool ClearBrowsingData(const std::string& target);
  bool ClearHistoryRange(const std::string& range);
  bool SetSetting(const std::string& key, const std::string& value);
  bool ResetSetting(const std::string& key);
  bool SetPermission(const std::string& origin, const std::string& permission,
                     const std::string& value);
  bool SetLiveSidebarWidth(double width);
  bool SetUiOverlayActive(bool active, double overlayWidth = 392.0,
                          double overlayHeight = 560.0);
  bool HandleSettingsUrl(const std::string &tabId, const std::string &url);
  bool HandleNewTabSearchUrl(const std::string &tabId, const std::string &url);
  // Polls pending HostCommands from FrostEngine and executes them, routing
  // host side effects (page/window I/O) back as HostEvents/results.
  void PollAndExecuteHostCommands();
  // Executes a single HostCommand JSON envelope. It does not acknowledge the
  // command: BrowserAppController owns exactly-once result delivery.
  bool ExecuteHostCommand(const std::string &commandJson,
                          std::string *error = nullptr);
  // Pushes a HostEvent JSON envelope back to FrostEngine.
  bool PushHostEventJson(const std::string &eventJson);
  std::string DownloadPathFor(const std::string &suggestedName) const;

  void SetUiBrowser(CefRefPtr<CefBrowser> browser);
  void OnTabBrowserCreated(const std::string& tabId, CefRefPtr<CefBrowser> browser);
  void OnTabTitle(const std::string& tabId, const std::string& title);
  void OnTabUrl(const std::string& tabId, const std::string& url);
  void OnTabFavicon(const std::string& tabId, const std::string& faviconUrl);
  void OnTabLoadingState(const std::string& tabId, bool isLoading, bool canGoBack,
                         bool canGoForward);
  void OnNavigationStarted(const std::string& tabId);
  void OnNavigationFinished(const std::string& tabId);
  void OnNavigationFailed(const std::string& tabId, const std::string& message);
  void ExpirePendingPopupTab(const std::string& tabId);
  CefWindowInfo PopupWindowInfo() const;
  void OnDownloadStarted(const std::string& downloadId, const std::string& url,
                         const std::string& path);
  void OnDownloadUpdated(const std::string& downloadId, const std::string& url,
                         const std::string& path, const std::string& state, int percent);
  void OnUiDraggableRegionsChanged(const std::vector<CefDraggableRegion>& regions);

  NativeBridge* Bridge() {
    return bridge_.get();
  }
  CommandRegistry& Commands() {
    return commands_;
  }
  TabManager& Tabs() {
    return tabManager_;
  }
  const TabManager& Tabs() const {
    return tabManager_;
  }
  FrostStore &Store();
  const FrostStore &Store() const;
  // Reads settings from this window's FrostRuntime. Private windows therefore
  // consult only their isolated in-memory engine, never the profile DB.
  std::string EngineSetting(const std::string &key,
                            const std::string &fallback = "") const;
  bool IsApprovedDownloadPath(const std::string &path) const;
  BrowserAppController& App() {
    return app_;
  }
  const BrowserAppController& App() const {
    return app_;
  }
  CefRefPtr<CefBrowser> UiBrowser() const {
    return uiBrowser_;
  }
  void UpdateContentFrame();
  std::string WindowId() const {
    return windowId_;
  }
  bool IsPrivate() const {
    return privateWindow_;
  }
  CefRefPtr<CefDictionaryValue> SessionSnapshot() const;

 private:
  struct ClosedTab {
    std::string title;
    std::string url;
    std::string faviconUrl;
    bool pinned = false;
  };

  void CreateNativeWindow();
  void CreateUiBrowser();
  void CreateTabBrowser(const Tab& tab);
  bool CreateRestoredTab(CefRefPtr<CefDictionaryValue> tabState, bool active);
  void RegisterCommands();
  void WireEvents();
  void ResizeViews();
  void UpdateTabPatch(const std::string& tabId, const std::string& title, const std::string& url,
                      bool isLoading, bool canGoBack, bool canGoForward);
  void SetActiveContentView();
  CefWindowHandle ContentParentHandle() const;
  CefWindowHandle UiParentHandle() const;

  BrowserAppController& app_;
  EventBus& eventBus_;
  TabManager& tabManager_;
  CommandRegistry commands_;
  std::unique_ptr<NativeBridge> bridge_;
  CefRefPtr<CefBrowser> uiBrowser_;
  CefRefPtr<CefRequestContext> privateRequestContext_;
  std::vector<ClosedTab> closedTabs_;
  double liveSidebarWidth_ = 0.0;
  std::vector<std::pair<EventType, int>> eventSubscriptions_;
  std::string windowId_;
  NSWindow* window_ = nullptr;
  NSView* uiHostView_ = nullptr;
  NSView* contentHostView_ = nullptr;
  NSView* dragRegionView_ = nullptr;
  bool uiOverlayActive_ = false;
  bool privateWindow_ = false;
  double uiOverlayWidth_ = 392.0;
  double uiOverlayHeight_ = 560.0;
};

}  // namespace fubuki
