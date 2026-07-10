#include "browser/BrowserWindow.h"

#import <Cocoa/Cocoa.h>
#import <QuartzCore/QuartzCore.h>

#include <algorithm>
#include <cstdlib>
#include <filesystem>
#include <map>

#include "browser/BrowserAppController.h"
#include "cef/FubukiClient.h"
#include "cef/FubukiSchemeHandler.h"
#include "include/cef_cookie.h"
#include "include/cef_parser.h"
#include "include/cef_request_context_handler.h"
#include "include/wrapper/cef_helpers.h"
#include "utils/UrlUtils.h"

namespace {
constexpr CGFloat kMinSidebarWidth = 168.0;
constexpr CGFloat kDefaultSidebarWidth = 196.0;
constexpr CGFloat kMaxSidebarWidth = 280.0;
}  // namespace

@interface FubukiUiHostView : NSView
@property(nonatomic) BOOL overlayActive;
@property(nonatomic) CGFloat sidebarWidth;
@property(nonatomic) CGFloat navHeight;
@property(nonatomic) CGFloat overlayWidth;
@property(nonatomic) CGFloat overlayHeight;
- (BOOL)isInteractivePoint:(NSPoint)point;
@end

@interface FubukiDragRegionView : NSView
@property(nonatomic) CGFloat sidebarWidth;
@property(nonatomic) CGFloat navHeight;
- (void)setDraggableRegions:(const std::vector<CefDraggableRegion>&)regions
              contentHeight:(CGFloat)height;
@end

namespace fubuki {
class BrowserWindow;
BrowserWindow* GetActiveBrowserWindow();
BrowserWindow* GetBrowserWindowForNativeWindow(NSWindow* window);
}  // namespace fubuki

@interface FubukiWindowDelegate : NSObject <NSWindowDelegate>
@end

@implementation FubukiUiHostView

- (BOOL)isOpaque {
  return NO;
}

- (BOOL)isInteractivePoint:(NSPoint)point {
  const NSRect bounds = [self bounds];
  if (point.y >= bounds.size.height - self.navHeight) {
    return YES;
  }
  if (self.sidebarWidth > 0.0 && point.x <= self.sidebarWidth) {
    return YES;
  }
  if (self.overlayActive) {
    const CGFloat panelWidth =
        std::min<CGFloat>(self.overlayWidth, std::max<CGFloat>(0.0, bounds.size.width - 16.0));
    const CGFloat panelHeight = std::min<CGFloat>(
        self.overlayHeight, std::max<CGFloat>(0.0, bounds.size.height - self.navHeight - 16.0));
    const NSRect panelRect = NSMakeRect(bounds.size.width - panelWidth - 8.0,
                                        bounds.size.height - self.navHeight - panelHeight - 8.0,
                                        panelWidth, panelHeight);
    if (NSPointInRect(point, panelRect)) {
      return YES;
    }
  }
  return NO;
}

- (NSView*)hitTest:(NSPoint)point {
  if (![self isInteractivePoint:point]) {
    return nil;
  }
  return [super hitTest:point];
}
@end

@implementation FubukiDragRegionView {
  NSMutableArray<NSValue*>* draggableRects_;
  NSMutableArray<NSValue*>* blockedRects_;
  CGFloat contentHeight_;
}

- (instancetype)initWithFrame:(NSRect)frame {
  self = [super initWithFrame:frame];
  if (self) {
    draggableRects_ = [[NSMutableArray alloc] init];
    blockedRects_ = [[NSMutableArray alloc] init];
    contentHeight_ = frame.size.height;
    self.sidebarWidth = kDefaultSidebarWidth;
    self.navHeight = 48.0;
    [self setAutoresizingMask:NSViewWidthSizable | NSViewMinYMargin];
  }
  return self;
}

- (void)dealloc {
  [draggableRects_ release];
  [blockedRects_ release];
  [super dealloc];
}

- (BOOL)isOpaque {
  return NO;
}

- (void)setDraggableRegions:(const std::vector<CefDraggableRegion>&)regions
              contentHeight:(CGFloat)height {
  [draggableRects_ removeAllObjects];
  [blockedRects_ removeAllObjects];
  contentHeight_ = height;
  for (const auto& region : regions) {
    const NSRect rect = NSMakeRect(region.bounds.x, height - region.bounds.y - region.bounds.height,
                                   region.bounds.width, region.bounds.height);
    [(region.draggable ? draggableRects_ : blockedRects_) addObject:[NSValue valueWithRect:rect]];
  }
}

- (NSView*)hitTest:(NSPoint)point {
  const NSView* hit = [super hitTest:point];
  if (hit != self) {
    return nil;
  }

  const NSPoint localPoint = point;
  if (localPoint.x >= self.sidebarWidth && localPoint.y <= contentHeight_ - self.navHeight) {
    return nil;
  }
  if (localPoint.y >= contentHeight_ - self.navHeight &&
      localPoint.x >= [self bounds].size.width - 280.0) {
    return nil;
  }
  for (NSValue* value in blockedRects_) {
    if (NSPointInRect(localPoint, [value rectValue])) {
      return nil;
    }
  }
  for (NSValue* value in draggableRects_) {
    if (NSPointInRect(localPoint, [value rectValue])) {
      return self;
    }
  }
  return nil;
}

- (void)mouseDown:(NSEvent*)event {
  if ([event clickCount] == 2) {
    const NSString* action =
        [[NSUserDefaults standardUserDefaults] stringForKey:@"AppleActionOnDoubleClick"];
    if ([action isEqualToString:@"Minimize"]) {
      [[self window] miniaturize:nil];
    } else if (![action isEqualToString:@"None"]) {
      [[self window] performZoom:nil];
    }
    return;
  }
  [[self window] performWindowDragWithEvent:event];
}
@end

@implementation FubukiWindowDelegate
- (void)windowDidBecomeKey:(NSNotification*)notification {
  NSWindow* window = (NSWindow*)[notification object];
  if (auto* browserWindow = fubuki::GetBrowserWindowForNativeWindow(window)) {
    browserWindow->App().NotifyWindowFocused(browserWindow);
  }
  for (NSButton* button in @[
         [window standardWindowButton:NSWindowCloseButton],
         [window standardWindowButton:NSWindowMiniaturizeButton],
         [window standardWindowButton:NSWindowZoomButton],
       ]) {
    [button setHidden:NO];
    [button setAlphaValue:1.0];
  }
}

- (void)windowDidResignKey:(NSNotification*)notification {
  NSWindow* window = (NSWindow*)[notification object];
  for (NSButton* button in @[
         [window standardWindowButton:NSWindowCloseButton],
         [window standardWindowButton:NSWindowMiniaturizeButton],
         [window standardWindowButton:NSWindowZoomButton],
       ]) {
    [button setHidden:NO];
    [button setAlphaValue:1.0];
  }
}

- (void)windowDidResize:(NSNotification*)notification {
  NSWindow* window = (NSWindow*)[notification object];
  if (auto* browserWindow = fubuki::GetBrowserWindowForNativeWindow(window)) {
    browserWindow->UpdateContentFrame();
  }
}

- (void)windowDidMove:(NSNotification*)notification {
  NSWindow* window = (NSWindow*)[notification object];
  if (auto* browserWindow = fubuki::GetBrowserWindowForNativeWindow(window)) {
    browserWindow->App().PersistSession();
  }
}

- (void)windowWillClose:(NSNotification*)notification {
  NSWindow* window = (NSWindow*)[notification object];
  if (auto* browserWindow = fubuki::GetBrowserWindowForNativeWindow(window)) {
    browserWindow->App().NotifyWindowClosed(browserWindow);
  }
}
@end

namespace fubuki {

FrostStore &BrowserWindow::Store() {
  return app_.Store();
}

const FrostStore &BrowserWindow::Store() const {
  return app_.Store();
}

namespace {

constexpr CGFloat kNavHeight = 42.0;
constexpr CGFloat kMinWidth = 900.0;
constexpr CGFloat kMinHeight = 620.0;

CefWindowInfo ChildWindowInfo(NSView* parent) {
  CefWindowInfo info;
  NSRect bounds = [parent bounds];
  info.SetAsChild(parent, CefRect(0, 0, static_cast<int>(bounds.size.width),
                                  static_cast<int>(bounds.size.height)));
  return info;
}

void MakeViewTreeTransparent(NSView* view) {
  if (!view) {
    return;
  }
  [view setWantsLayer:YES];
  if (view.layer) {
    view.layer.opaque = NO;
    view.layer.backgroundColor = [[NSColor clearColor] CGColor];
  }
  if ([view respondsToSelector:@selector(setDrawsBackground:)]) {
    [(id)view setDrawsBackground:NO];
  }
  for (NSView* subview in [view subviews]) {
    MakeViewTreeTransparent(subview);
  }
}

void SetBrowserViewHidden(CefRefPtr<CefBrowser> browser, bool hidden) {
  if (!browser) {
    return;
  }
  NSView* view = reinterpret_cast<NSView*>(browser->GetHost()->GetWindowHandle());
  [view setHidden:hidden];
}

void UpdateUiHostClip(NSView* view, bool overlayActive, CGFloat sidebarWidth, CGFloat navHeight,
                      CGFloat overlayWidth, CGFloat overlayHeight) {
  if (!view || !view.layer) {
    return;
  }
  const NSRect bounds = [view bounds];
  CGMutablePathRef path = CGPathCreateMutable();
  CGPathAddRect(path, nullptr,
                CGRectMake(0.0, bounds.size.height - navHeight, bounds.size.width, navHeight));
  if (sidebarWidth > 0.0) {
    CGPathAddRect(path, nullptr, CGRectMake(0.0, 0.0, sidebarWidth, bounds.size.height));
  }
  if (overlayActive) {
    const CGFloat panelWidth =
        std::min<CGFloat>(overlayWidth, std::max<CGFloat>(0.0, bounds.size.width - 16.0));
    const CGFloat panelHeight = std::min<CGFloat>(
        overlayHeight, std::max<CGFloat>(0.0, bounds.size.height - navHeight - 16.0));
    CGPathAddRect(
        path, nullptr,
        CGRectMake(bounds.size.width - panelWidth - 8.0,
                   bounds.size.height - navHeight - panelHeight - 8.0, panelWidth, panelHeight));
  }
  CAShapeLayer* mask = [CAShapeLayer layer];
  mask.frame = bounds;
  mask.path = path;
  view.layer.mask = mask;
  CGPathRelease(path);
}

bool IsSafeSettingsReturnPage(const std::string& returnPage) {
  if (returnPage.empty()) {
    return true;
  }
  if (returnPage.rfind("set", 0) == 0 || returnPage.rfind("/set", 0) == 0 ||
      returnPage.rfind("fubuki://settings/set", 0) == 0) {
    return false;
  }
  return returnPage.rfind("fubuki://", 0) == 0 || returnPage.find("://") == std::string::npos;
}

std::string QueryParam(const std::string& url, const std::string& key) {
  const size_t queryStart = url.find('?');
  if (queryStart == std::string::npos) {
    return "";
  }
  const std::string query = url.substr(queryStart + 1);
  size_t start = 0;
  while (start <= query.size()) {
    const size_t end = query.find('&', start);
    const std::string pair =
        query.substr(start, end == std::string::npos ? std::string::npos : end - start);
    const size_t equals = pair.find('=');
    const std::string name = equals == std::string::npos ? pair : pair.substr(0, equals);
    if (name == key) {
      const std::string value = equals == std::string::npos ? "" : pair.substr(equals + 1);
      return CefURIDecode(
                 value, true,
                 static_cast<cef_uri_unescape_rule_t>(UU_SPACES | UU_PATH_SEPARATORS |
                                                      UU_URL_SPECIAL_CHARS_EXCEPT_PATH_SEPARATORS |
                                                      UU_REPLACE_PLUS_WITH_SPACE))
          .ToString();
    }
    if (end == std::string::npos) {
      break;
    }
    start = end + 1;
  }
  return "";
}

void RegisterFubukiSchemeHandlers(CefRefPtr<CefRequestContext> context) {
  if (!context) {
    return;
  }
  for (const char* host :
       {"app", "newtab", "settings", "bookmarks", "downloads", "history", "debug"}) {
    context->RegisterSchemeHandlerFactory("fubuki", host,
                                          new FubukiSchemeHandlerFactory(FUBUKI_UI_DIST));
  }
}

}  // namespace

BrowserWindow* gActiveBrowserWindow = nullptr;
std::map<NSWindow*, BrowserWindow*> gBrowserWindowsByNativeWindow;
FubukiWindowDelegate* gWindowDelegate = nil;

BrowserWindow* GetActiveBrowserWindow() {
  return gActiveBrowserWindow;
}

BrowserWindow* GetBrowserWindowForNativeWindow(NSWindow* window) {
  auto it = gBrowserWindowsByNativeWindow.find(window);
  return it == gBrowserWindowsByNativeWindow.end() ? nullptr : it->second;
}

BrowserWindow::BrowserWindow(BrowserAppController& app, TabManager& tabManager,
                             std::string windowId, bool privateWindow)
    : app_(app),
      eventBus_(app.Events()),
      tabManager_(tabManager),
      bridge_(std::make_unique<NativeBridge>(*this, app.Engine())),
      windowId_(std::move(windowId)),
      privateWindow_(privateWindow) {
  gActiveBrowserWindow = this;
  if (!privateWindow_) {
    Store().AddLog("info", "BrowserWindow initialized");
  }
  if (privateWindow_) {
    CefRequestContextSettings settings;
    privateRequestContext_ = CefRequestContext::CreateContext(settings, nullptr);
    RegisterFubukiSchemeHandlers(privateRequestContext_);
  }
  RegisterCommands();
  WireEvents();
}

BrowserWindow::~BrowserWindow() {
  for (const auto& [type, token] : eventSubscriptions_) {
    eventBus_.Unsubscribe(type, token);
  }
  if (window_) {
    gBrowserWindowsByNativeWindow.erase(window_);
  }
  if (gActiveBrowserWindow == this) {
    gActiveBrowserWindow = nullptr;
  }
}

void BrowserWindow::Show(CefRefPtr<CefDictionaryValue> restoreState) {
  CEF_REQUIRE_UI_THREAD();
  CreateNativeWindow();
  if (restoreState && restoreState->HasKey("frame") &&
      restoreState->GetType("frame") == VTYPE_DICTIONARY) {
    auto frameDict = restoreState->GetDictionary("frame");
    const CGFloat width = std::max<CGFloat>(kMinWidth, frameDict->GetDouble("width"));
    const CGFloat height = std::max<CGFloat>(kMinHeight, frameDict->GetDouble("height"));
    [window_
        setFrame:NSMakeRect(frameDict->GetDouble("x"), frameDict->GetDouble("y"), width, height)
         display:NO];
  }
  CreateUiBrowser();
  bool restored = false;
  if (restoreState && restoreState->HasKey("tabs") && restoreState->GetType("tabs") == VTYPE_LIST) {
    auto tabs = restoreState->GetList("tabs");
    for (size_t i = 0; i < tabs->GetSize(); ++i) {
      if (auto tabState = tabs->GetDictionary(i)) {
        const bool active = tabState->HasKey("active") && tabState->GetBool("active");
        restored = CreateRestoredTab(tabState, active) || restored;
      }
    }
  }
  // Tab creation is fully owned by FrostEngine via the page.create host
  // command queue. BrowserAppController::Start() already enqueues the
  // initial tabs.create request, and subsequent windows get tabs through
  // the engine's WindowsCreate handler. We must NOT call
  // bridge_->Invoke("tabs.create") here to avoid duplicate tabs.
  [window_ makeKeyAndOrderFront:nil];
}

bool BrowserWindow::CloseWindow() {
  if (!window_) {
    return false;
  }
  [window_ performClose:nil];
  return true;
}

bool BrowserWindow::CreateTab(const std::string& input, bool active) {
  const std::string url = NormalizeNavigationInput(input,
                                                   Store().GetSetting("searchEngine"),
                                                   Store().GetSetting("customSearchUrl"));
  Tab& tab = tabManager_.CreateTab(url, active);
  CreateTabBrowser(tab);
  ResizeViews();
  SetActiveContentView();
  if (!privateWindow_) {
    app_.PersistSession();
  }
  return true;
}

bool BrowserWindow::CreateTabWithId(const std::string& input,
                                    const std::string& tabId, bool active) {
  const std::string url = NormalizeNavigationInput(input,
                                                   Store().GetSetting("searchEngine"),
                                                   Store().GetSetting("customSearchUrl"));
  Tab& tab = tabManager_.CreateTab(url, active, tabId);
  CreateTabBrowser(tab);
  ResizeViews();
  SetActiveContentView();
  if (!privateWindow_) {
    app_.PersistSession();
  }
  return true;
}

std::string BrowserWindow::CreatePendingPopupTab(const std::string& url, bool active) {
  Tab& tab = tabManager_.CreateTab(url.empty() ? "about:blank" : url, active);
  tab.isPendingPopup = true;
  tabManager_.UpdateTab(tab.id, tab);
  ResizeViews();
  SetActiveContentView();
  if (!privateWindow_) {
    app_.PersistSession();
  }
  return tab.id;
}

bool BrowserWindow::ActivateTab(const std::string& tabId) {
  const bool ok = tabManager_.ActivateTab(tabId);
  SetActiveContentView();
  if (ok && !privateWindow_) {
    app_.PersistSession();
  }
  return ok;
}

bool BrowserWindow::CloseTab(const std::string& tabId) {
  Tab* tab = tabManager_.GetTab(tabId);
  if (tab && !privateWindow_) {
    closedTabs_.push_back({tab->title, tab->url, tab->faviconUrl, tab->isPinned});
    if (closedTabs_.size() > 30) {
      closedTabs_.erase(closedTabs_.begin());
    }
  }
  if (tab && tab->browser) {
    NSView* view = reinterpret_cast<NSView*>(tab->browser->GetHost()->GetWindowHandle());
    [view removeFromSuperview];
    tab->browser->GetHost()->CloseBrowser(true);
    tab->browser = nullptr;
  }
  const bool ok = tabManager_.CloseTab(tabId);
  for (const auto& item : tabManager_.GetTabs()) {
    if (!item.browser) {
      if (Tab* newTab = tabManager_.GetTab(item.id)) {
        CreateTabBrowser(*newTab);
      }
    }
  }
  ResizeViews();
  SetActiveContentView();
  if (!privateWindow_) {
    app_.PersistSession();
  }
  return ok;
}

bool BrowserWindow::PinTab(const std::string& tabId, bool pinned) {
  const bool ok = tabManager_.SetPinned(tabId, pinned);
  if (ok && !privateWindow_) {
    app_.PersistSession();
  }
  return ok;
}

bool BrowserWindow::DuplicateTab(const std::string& tabId) {
  Tab* tab = tabManager_.GetTab(tabId);
  if (!tab) {
    return false;
  }
  return CreateTab(tab->url.empty() ? "fubuki://newtab/" : tab->url, true);
}

bool BrowserWindow::ReopenClosedTab() {
  if (closedTabs_.empty()) {
    return false;
  }
  ClosedTab closed = closedTabs_.back();
  closedTabs_.pop_back();
  if (!CreateTab(closed.url.empty() ? "fubuki://newtab/" : closed.url, true)) {
    return false;
  }
  if (Tab* tab = tabManager_.GetActiveTab()) {
    tabManager_.SetPinned(tab->id, closed.pinned);
  }
  return true;
}

bool BrowserWindow::CloseOtherTabs(const std::string& tabId) {
  bool changed = false;
  const auto tabs = tabManager_.GetTabs();
  for (const auto& tab : tabs) {
    if (tab.id != tabId && !tab.isPinned) {
      changed = CloseTab(tab.id) || changed;
    }
  }
  return changed;
}

bool BrowserWindow::CloseTabsToRight(const std::string& tabId) {
  const auto tabs = tabManager_.GetTabs();
  auto it = std::find_if(tabs.begin(), tabs.end(), [&](const Tab& tab) { return tab.id == tabId; });
  if (it == tabs.end()) {
    return false;
  }
  bool close = false;
  bool changed = false;
  for (const auto& tab : tabs) {
    if (close && !tab.isPinned) {
      changed = CloseTab(tab.id) || changed;
    }
    if (tab.id == tabId) {
      close = true;
    }
  }
  return changed;
}

bool BrowserWindow::MoveTab(const std::string& tabId, int toIndex) {
  const bool ok = tabManager_.MoveTab(tabId, static_cast<size_t>(std::max(0, toIndex)));
  if (ok && !privateWindow_) {
    app_.PersistSession();
  }
  return ok;
}

bool BrowserWindow::MoveTabToNewWindow(const std::string& tabId) {
  Tab* tab = tabManager_.GetTab(tabId);
  if (!tab) {
    return false;
  }
  auto windowState = CefDictionaryValue::Create();
  auto tabs = CefListValue::Create();
  auto item = CefDictionaryValue::Create();
  item->SetString("title", tab->title);
  item->SetString("url", tab->url);
  item->SetString("faviconUrl", tab->faviconUrl);
  item->SetBool("pinned", tab->isPinned);
  item->SetBool("active", true);
  tabs->SetDictionary(0, item);
  windowState->SetList("tabs", tabs);
  app_.RequestNewWindow(privateWindow_, windowState);
  CloseTab(tabId);
  return true;
}

bool BrowserWindow::Navigate(const std::string& tabId, const std::string& input) {
  Tab* tab = tabManager_.GetTab(tabId);
  if (!tab || !tab->browser) {
    return false;
  }
  tab->browser->GetMainFrame()->LoadURL(NormalizeNavigationInput(input,
                                                                  Store().GetSetting("searchEngine"),
                                                                  Store().GetSetting("customSearchUrl")));
  return true;
}

bool BrowserWindow::Reload(const std::string& tabId) {
  if (Tab* tab = tabManager_.GetTab(tabId); tab && tab->browser) {
    tab->browser->Reload();
    return true;
  }
  return false;
}

bool BrowserWindow::Stop(const std::string& tabId) {
  if (Tab* tab = tabManager_.GetTab(tabId); tab && tab->browser) {
    tab->browser->StopLoad();
    return true;
  }
  return false;
}

bool BrowserWindow::GoBack(const std::string& tabId) {
  if (Tab* tab = tabManager_.GetTab(tabId); tab && tab->browser && tab->browser->CanGoBack()) {
    tab->browser->GoBack();
    return true;
  }
  return false;
}

bool BrowserWindow::GoForward(const std::string& tabId) {
  if (Tab* tab = tabManager_.GetTab(tabId); tab && tab->browser && tab->browser->CanGoForward()) {
    tab->browser->GoForward();
    return true;
  }
  return false;
}

bool BrowserWindow::GoHome() {
  Tab* tab = tabManager_.GetActiveTab();
  if (!tab) {
    return CreateTab(Store().GetSetting("homeUrl"), true);
  }
  const std::string home = Store().GetSetting("homeUrl").empty()
                               ? Store().GetSetting("homepage")
                               : Store().GetSetting("homeUrl");
  return Navigate(tab->id, home);
}

bool BrowserWindow::FindInPage(const std::string& query, bool forward) {
  Tab* tab = tabManager_.GetActiveTab();
  if (!tab || !tab->browser || query.empty()) {
    return false;
  }
  tab->browser->GetHost()->Find(query, forward, false, false);
  return true;
}

bool BrowserWindow::StopFinding(bool clearSelection) {
  Tab* tab = tabManager_.GetActiveTab();
  if (!tab || !tab->browser) {
    return false;
  }
  tab->browser->GetHost()->StopFinding(clearSelection);
  return true;
}

bool BrowserWindow::ZoomIn() {
  Tab* tab = tabManager_.GetActiveTab();
  if (!tab || !tab->browser) {
    return false;
  }
  tab->zoomLevel = std::min(9.0, tab->zoomLevel + 0.5);
  tab->browser->GetHost()->SetZoomLevel(tab->zoomLevel);
  tabManager_.UpdateTab(tab->id, *tab);
  return true;
}

bool BrowserWindow::ZoomOut() {
  Tab* tab = tabManager_.GetActiveTab();
  if (!tab || !tab->browser) {
    return false;
  }
  tab->zoomLevel = std::max(-7.0, tab->zoomLevel - 0.5);
  tab->browser->GetHost()->SetZoomLevel(tab->zoomLevel);
  tabManager_.UpdateTab(tab->id, *tab);
  return true;
}

bool BrowserWindow::ResetZoom() {
  Tab* tab = tabManager_.GetActiveTab();
  if (!tab || !tab->browser) {
    return false;
  }
  tab->zoomLevel = 0.0;
  tab->browser->GetHost()->SetZoomLevel(0.0);
  tabManager_.UpdateTab(tab->id, *tab);
  return true;
}

bool BrowserWindow::PrintPage() {
  Tab* tab = tabManager_.GetActiveTab();
  if (!tab || !tab->browser) {
    return false;
  }
  tab->browser->GetHost()->Print();
  return true;
}

bool BrowserWindow::ViewSource() {
  Tab* tab = tabManager_.GetActiveTab();
  if (!tab || tab->url.empty() || tab->url.rfind("fubuki://", 0) == 0 ||
      tab->url.rfind("data:", 0) == 0) {
    return false;
  }
  return CreateTab("view-source:" + tab->url, true);
}

bool BrowserWindow::FocusOmnibox() {
  if (!uiBrowser_) {
    return false;
  }
  uiBrowser_->GetHost()->SetFocus(true);
  uiBrowser_->GetMainFrame()->ExecuteJavaScript(
      "document.querySelector('.omnibox input')?.select();", "fubuki://app/", 0);
  return true;
}

CefRefPtr<CefValue> BrowserWindow::ExecuteCommand(const std::string& commandId,
                                                  CefRefPtr<CefDictionaryValue> args) {
  return commands_.Execute(commandId, args);
}

bool BrowserWindow::HandleShortcut(bool commandDown, bool altDown, int keyCode, char character) {
  Tab* tab = tabManager_.GetActiveTab();
  const std::string tabId = tab ? tab->id : "";
  if ((commandDown && character == 'l') || (commandDown && character == 'L')) {
    return FocusOmnibox();
  }
  if (commandDown && character == 'N') {
    return GetBrowserAppController() ? GetBrowserAppController()->RequestNewPrivateWindow() : false;
  }
  if (commandDown && character == 'n') {
    return GetBrowserAppController() ? GetBrowserAppController()->RequestNewWindow(false, nullptr)
                                     : false;
  }
  if (commandDown && character == ',') {
    return tab ? Navigate(tabId, "fubuki://settings/") : CreateTab("fubuki://settings/", true);
  }
  if (commandDown && (character == 'b' || character == 'B')) {
    const std::string current = Store().GetSetting("sidebarVisible");
    return SetSetting("sidebarVisible", current == "hide" ? "show" : "hide");
  }
  if (commandDown && (character == 'd' || character == 'D')) {
    if (uiBrowser_) {
      uiBrowser_->GetMainFrame()->ExecuteJavaScript(
          "window.dispatchEvent(new CustomEvent('fubuki:toggle-active-bookmark'));",
          "fubuki://app/", 0);
      return true;
    }
    return false;
  }
  if (!tab) {
    return false;
  }
  if (commandDown && character == 'T') {
    return ReopenClosedTab();
  }
  if (commandDown && character == '+') {
    return ZoomIn();
  }
  if (commandDown && character == '-') {
    return ZoomOut();
  }
  if (commandDown && character == '0') {
    return ResetZoom();
  }
  if (commandDown && (character == 'f' || character == 'F')) {
    if (uiBrowser_) {
      uiBrowser_->GetMainFrame()->ExecuteJavaScript(
          "window.dispatchEvent(new CustomEvent('fubuki:show-find'));", "fubuki://app/", 0);
      return true;
    }
    return false;
  }
  if (commandDown && (character == 'r' || character == 'R')) {
    return Reload(tabId);
  }
  if (commandDown && character == 't') {
    return CreateTab("fubuki://newtab/", true);
  }
  if (commandDown && (character == 'w' || character == 'W')) {
    return CloseTab(tabId);
  }
  if ((commandDown && character == '[') || (altDown && keyCode == 0x25)) {
    return GoBack(tabId);
  }
  if ((commandDown && character == ']') || (altDown && keyCode == 0x27)) {
    return GoForward(tabId);
  }
  return false;
}

bool BrowserWindow::OpenDevTools() {
  Tab* tab = tabManager_.GetActiveTab();
  CefRefPtr<CefBrowser> browser = tab && tab->browser ? tab->browser : uiBrowser_;
  if (!browser) {
    return false;
  }

  CefRefPtr<CefBrowserHost> host = browser->GetHost();
  if (!host) {
    return false;
  }

  CefWindowInfo info;
  CefString(&info.window_name).FromString("Fubuki DevTools");
  info.bounds = CefRect(160, 160, 1000, 720);
  info.hidden = false;
  CefBrowserSettings settings;
  host->ShowDevTools(info, nullptr, settings, CefPoint());
  return true;
}

bool BrowserWindow::AddActiveBookmark() {
  Tab* tab = tabManager_.GetActiveTab();
  if (!tab) {
    return false;
  }
  const bool ok = Store().AddBookmark(tab->title, tab->url, tab->faviconUrl);
  Store().AddLog("info", "Bookmark added: " + tab->url);
  eventBus_.Publish({EventType::BookmarkChanged, "bookmark.changed", *tab, windowId_, tab->id, tab->url});
  return ok;
}

bool BrowserWindow::SaveBookmark(const std::string& title, const std::string& url, const std::string& faviconUrl) {
  const bool ok = Store().AddBookmark(title, url, faviconUrl);
  eventBus_.Publish({EventType::BookmarkChanged, "bookmark.changed", {}, windowId_, "", url});
  PageCache::Instance().Invalidate("fubuki://bookmarks");
  return ok;
}

bool BrowserWindow::RemoveBookmark(const std::string& url) {
  const bool ok = Store().RemoveBookmark(url);
  eventBus_.Publish({EventType::BookmarkChanged, "bookmark.changed", {}, windowId_, "", url});
  PageCache::Instance().Invalidate("fubuki://bookmarks");
  return ok;
}

bool BrowserWindow::RemoveHistory(const std::string& url) {
  const bool ok = Store().RemoveHistory(url);
  eventBus_.Publish({EventType::HistoryChanged, "history.changed", {}, windowId_, "", url});
  PageCache::Instance().Invalidate("fubuki://history");
  return ok;
}

bool BrowserWindow::RemoveDownload(const std::string& url, const std::string& path) {
  const bool ok = Store().RemoveDownload(url, path);
  PageCache::Instance().Invalidate("fubuki://downloads");
  eventBus_.Publish({EventType::DownloadChanged, "download.changed", {}, windowId_, "", path.empty() ? url : path});
  return ok;
}

bool BrowserWindow::OpenDownloadedFile(const std::string& path) {
  if (path.empty() || !Store().HasDownloadPath(path) || !std::filesystem::exists(path)) {
    return false;
  }
  NSURL* url = [NSURL fileURLWithPath:[NSString stringWithUTF8String:path.c_str()]];
  return [[NSWorkspace sharedWorkspace] openURL:url];
}

bool BrowserWindow::RevealDownloadedFile(const std::string& path) {
  if (path.empty() || !Store().HasDownloadPath(path) || !std::filesystem::exists(path)) {
    return false;
  }
  NSURL* url = [NSURL fileURLWithPath:[NSString stringWithUTF8String:path.c_str()]];
  [[NSWorkspace sharedWorkspace] activateFileViewerSelectingURLs:@[ url ]];
  return true;
}

bool BrowserWindow::ClearBrowsingData(const std::string& target) {
  bool ok = false;
  if (target == "bookmarks") {
    ok = Store().ClearBookmarks();
  } else if (target == "history") {
    ok = Store().ClearHistory();
  } else if (target == "downloads") {
    ok = Store().ClearDownloads();
  } else if (target == "logs") {
    ok = Store().ClearLogs();
  } else if (target == "cookies" || target == "cache" || target == "siteData") {
    CefRefPtr<CefCookieManager> cookieManager = CefCookieManager::GetGlobalManager(nullptr);
    CefRefPtr<CefRequestContext> context = CefRequestContext::GetGlobalContext();
    if ((target == "cookies" || target == "siteData") && !cookieManager) {
      return false;
    }
    if ((target == "cache" || target == "siteData") && !context) {
      return false;
    }
    if (target == "cookies" || target == "siteData") {
      cookieManager->DeleteCookies("", "", nullptr);
    }
    if (target == "cache" || target == "siteData") {
      context->ClearHttpCache(nullptr);
    }
    ok = true;
  } else if (target == "all") {
    CefRefPtr<CefCookieManager> cookieManager = CefCookieManager::GetGlobalManager(nullptr);
    CefRefPtr<CefRequestContext> context = CefRequestContext::GetGlobalContext();
    if (!cookieManager || !context) {
      return false;
    }
    cookieManager->DeleteCookies("", "", nullptr);
    context->ClearHttpCache(nullptr);
    ok = Store().ClearHistory() && Store().ClearDownloads() && Store().ClearLogs();
  }
  if (ok) {
    if (target != "logs" && target != "all") {
      Store().AddLog("info", "Browsing data cleared: " + target);
    }
    if (target == "bookmarks") {
      eventBus_.Publish({EventType::BookmarkChanged, "bookmark.changed", {}, windowId_, "", "clear"});
    }
    if (target == "history" || target == "all") {
      eventBus_.Publish({EventType::HistoryChanged, "history.changed", {}, windowId_, "", "clear"});
    }
    if (target == "downloads" || target == "all") {
      eventBus_.Publish({EventType::DownloadChanged, "download.changed", {}, windowId_, "", "clear"});
    }
  }
  return ok;
}

bool BrowserWindow::ClearHistoryRange(const std::string& range) {
  const bool ok = Store().ClearHistoryRange(range);
  if (ok) {
    eventBus_.Publish(
        {EventType::HistoryChanged, "history.changed", {}, windowId_, "", "clear:" + range});
  }
  return ok;
}

bool BrowserWindow::SetSetting(const std::string& key, const std::string& value) {
  if (key != "homepage" && key != "downloadDirectory" && key != "searchEngine" &&
      key != "startupBehavior" && key != "theme" && key != "language" &&
      key != "newTabBackgroundMode" && key != "newTabBackgroundColor" &&
      key != "newTabBackgroundUrl" && key != "customSearchUrl" && key != "appearance" &&
      key != "toolbarDensity" && key != "sidebarVisible" && key != "sidebarWidth" &&
      key != "defaultBookmarkDisplay" && key != "openBookmarkIn" && key != "showBookmarkFavicons" &&
      key != "newTabPage" && key != "homeUrl" && key != "askBeforeDownload" &&
      key != "defaultZoomLevel" && key != "closeWindowWithLastTab" &&
      key != "privateSearchEngine") {
    return false;
  }
  std::string savedValue = value;
  if (key == "sidebarWidth") {
    try {
      savedValue = std::to_string(
          static_cast<int>(std::clamp(std::stod(value), static_cast<double>(kMinSidebarWidth),
                                      static_cast<double>(kMaxSidebarWidth))));
    } catch (...) {
      savedValue = std::to_string(static_cast<int>(kDefaultSidebarWidth));
    }
  }
  Store().SetSetting(key, savedValue);
  if (key == "sidebarWidth") {
    liveSidebarWidth_ = 0.0;
  }
  if (key == "sidebarVisible" || key == "sidebarWidth" || key == "toolbarDensity") {
    UpdateContentFrame();
  }
  if (!privateWindow_) {
    Store().AddLog("info", "Setting updated: " + key);
  }
  eventBus_.Publish({EventType::SettingChanged, "setting.changed", {}, windowId_, "", key});
  if (!privateWindow_) {
    app_.PersistSession();
  }
  PageCache::Instance().Invalidate("fubuki://settings");
  return true;
}

bool BrowserWindow::ResetSetting(const std::string& key) {
  Store().ResetSetting(key);
  if (key == "sidebarWidth") {
    liveSidebarWidth_ = 0.0;
  }
  if (key == "sidebarVisible" || key == "sidebarWidth" || key == "toolbarDensity") {
    UpdateContentFrame();
  }
  eventBus_.Publish({EventType::SettingChanged, "setting.changed", {}, windowId_, "", key});
  if (!privateWindow_) {
    app_.PersistSession();
  }
  PageCache::Instance().Invalidate("fubuki://settings");
  return true;
}

bool BrowserWindow::SetPermission(const std::string& origin, const std::string& permission, const std::string& value) {
  const bool ok = Store().SetPermission(origin, permission, value);
  if (ok) {
    eventBus_.Publish(
        {EventType::PermissionChanged, "permission.changed", {}, windowId_, "", origin});
  }
  return ok;
}

bool BrowserWindow::SetLiveSidebarWidth(double width) {
  liveSidebarWidth_ = std::clamp(width, static_cast<double>(kMinSidebarWidth),
                                 static_cast<double>(kMaxSidebarWidth));
  UpdateContentFrame();
  return true;
}

bool BrowserWindow::SetUiOverlayActive(bool active, double overlayWidth, double overlayHeight) {
  if (!uiHostView_ || !contentHostView_) {
    return false;
  }
  uiOverlayActive_ = active;
  uiOverlayWidth_ = overlayWidth;
  uiOverlayHeight_ = overlayHeight;
  NSView* root = [uiHostView_ superview];
  if (!root) {
    return false;
  }
  [uiHostView_ removeFromSuperview];
  [contentHostView_ removeFromSuperview];
  if (active) {
    [root addSubview:contentHostView_];
    [root addSubview:uiHostView_ positioned:NSWindowAbove relativeTo:contentHostView_];
  } else {
    [root addSubview:uiHostView_];
    [root addSubview:contentHostView_ positioned:NSWindowAbove relativeTo:uiHostView_];
  }
  if (dragRegionView_) {
    [dragRegionView_ removeFromSuperview];
    [root addSubview:dragRegionView_
          positioned:NSWindowAbove
          relativeTo:(active ? uiHostView_ : contentHostView_)];
  }
  UpdateContentFrame();
  return true;
}

bool BrowserWindow::HandleSettingsUrl(const std::string& tabId, const std::string& url) {
  const std::string key = QueryParam(url, "key");
  const std::string value = QueryParam(url, "value");
  const std::string returnPage = QueryParam(url, "return");
  bool ok = false;
  if (key == "removeBookmark") {
    ok = RemoveBookmark(value);
  } else if (key == "removeHistory") {
    ok = RemoveHistory(value);
  } else if (key == "removeDownload") {
    ok = RemoveDownload(value, value);
  } else if (key == "openDownload") {
    ok = OpenDownloadedFile(value);
  } else if (key == "revealDownload") {
    ok = RevealDownloadedFile(value);
  } else if (key == "openDevTools") {
    ok = OpenDevTools();
  } else if (key == "clearData") {
    ok = ClearBrowsingData(value);
  } else if (key == "clearHistoryRange") {
    ok = ClearHistoryRange(value);
  } else if (key == "resetSetting") {
    ok = ResetSetting(value);
  } else {
    ok = SetSetting(key, value);
  }
  if (ok && tabManager_.GetTab(tabId)) {
    const std::string safeReturnPage = IsSafeSettingsReturnPage(returnPage) ? returnPage : "";
    if (safeReturnPage.rfind("fubuki://", 0) == 0) {
      Navigate(tabId, safeReturnPage);
    } else {
      Navigate(tabId, safeReturnPage.empty() ? "fubuki://settings/"
                                             : "fubuki://settings/" + safeReturnPage);
    }
  }
  return ok;
}

bool BrowserWindow::HandleNewTabSearchUrl(const std::string& tabId, const std::string& url) {
  const std::string query = QueryParam(url, "q");
  if (query.empty()) {
    return false;
  }
  return Navigate(tabId, query);
}

namespace {

// Wraps a string as a JSON string value. CEF's JSON writer escapes it, so
// values like tabId/url/title can never produce broken JSON.
CefRefPtr<CefValue> JsonStringValue(const std::string& s) {
  auto v = CefValue::Create();
  v->SetString(s);
  return v;
}

CefRefPtr<CefValue> JsonBoolValue(bool b) {
  auto v = CefValue::Create();
  v->SetBool(b);
  return v;
}

CefRefPtr<CefValue> JsonIntValue(int n) {
  auto v = CefValue::Create();
  v->SetInt(n);
  return v;
}

// Builds a HostCommandResult JSON envelope for the given command id. Values are
// written through CEF's JSON writer so strings are always correctly escaped.
std::string HostCommandResultJson(const std::string& commandId, bool ok,
                                  const std::string& error) {
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

// Builds a HostEvent JSON envelope. Field values are passed as typed CefValue
// handles so that strings are always correctly JSON-escaped (no hand-built
// fragments that can produce broken JSON for tabId/url/etc.).
std::string HostEventJson(const std::string& event,
                          const std::map<std::string, CefRefPtr<CefValue>>& fields) {
  auto root = CefDictionaryValue::Create();
  root->SetInt("version", 0);
  root->SetString("event", event);
  auto payload = CefDictionaryValue::Create();
  for (const auto& kv : fields) {
    payload->SetValue(kv.first, kv.second);
  }
  root->SetDictionary("payload", payload);
  auto value = CefValue::Create();
  value->SetDictionary(root);
  return CefWriteJSON(value, JSON_WRITER_DEFAULT).ToString();
}

std::string JsonString(const CefRefPtr<CefDictionaryValue>& dict,
                       const std::string& key, const std::string& fallback = "") {
  return dict->HasKey(key) && dict->GetType(key) == VTYPE_STRING
             ? dict->GetString(key).ToString()
             : fallback;
}

bool JsonBool(const CefRefPtr<CefDictionaryValue>& dict, const std::string& key) {
  return dict->HasKey(key) && dict->GetBool(key);
}

}  // namespace

bool BrowserWindow::PushHostEventJson(const std::string& eventJson) {
  return bridge_->PushHostEventJson(eventJson);
}

bool BrowserWindow::ExecuteHostCommand(const std::string& commandJson) {
  CefRefPtr<CefValue> value = CefParseJSON(commandJson, JSON_PARSER_ALLOW_TRAILING_COMMAS);
  if (!value || value->GetType() != VTYPE_DICTIONARY) {
    return false;
  }
  CefRefPtr<CefDictionaryValue> envelope = value->GetDictionary();
  const std::string commandId = JsonString(envelope, "id");
  const std::string command = JsonString(envelope, "command");
  CefRefPtr<CefDictionaryValue> payload =
      envelope->HasKey("payload")
          ? envelope->GetDictionary("payload")
          : CefDictionaryValue::Create();

  bool ok = false;
  std::string error;
  if (command == "page.create") {
    const std::string tabId = JsonString(payload, "tabId");
    const std::string url = JsonString(payload, "url");
    const bool active = JsonBool(payload, "active");
    ok = !tabId.empty() &&
         CreateTabWithId(url.empty() ? "fubuki://newtab/" : url, tabId, active);
    if (ok) {
      PushHostEventJson(HostEventJson("page.created",
                                      {{ "tabId", JsonStringValue(tabId) },
                                       { "windowId", JsonStringValue(windowId_) },
                                       { "url", JsonStringValue(url) }}));
    } else {
      error = "failed to create page";
    }
  } else if (command == "page.close") {
    const std::string tabId = JsonString(payload, "tabId");
    const std::string successorTabId = JsonString(payload, "successorTabId");
    const bool validSuccessor = successorTabId.empty() ||
                                (successorTabId != tabId &&
                                 tabManager_.GetTab(successorTabId));
    if (validSuccessor) {
      ok = CloseTab(tabId);
      if (ok && !successorTabId.empty()) {
        ok = ActivateTab(successorTabId);
      }
    }
    if (!ok) {
      error = validSuccessor ? "failed to close page or activate successor"
                             : "invalid successor tab";
    }
  } else if (command == "page.activate") {
    ok = ActivateTab(JsonString(payload, "tabId"));
    if (!ok) {
      error = "unknown tab";
    }
  } else if (command == "page.pin") {
    ok = PinTab(JsonString(payload, "tabId"), JsonBool(payload, "pinned"));
    if (!ok) {
      error = "unknown tab";
    }
  } else if (command == "page.move") {
    ok = MoveTab(JsonString(payload, "tabId"),
                 payload->HasKey("toIndex") ? payload->GetInt("toIndex") : 0);
    if (!ok) {
      error = "unknown tab";
    }
  } else if (command == "page.navigate") {
    ok = Navigate(JsonString(payload, "tabId"), JsonString(payload, "url"));
    if (!ok) {
      error = "unknown tab";
    }
  } else if (command == "page.reload") {
    ok = Reload(JsonString(payload, "tabId"));
    if (!ok) {
      error = "unknown tab";
    }
  } else if (command == "page.stop") {
    ok = Stop(JsonString(payload, "tabId"));
    if (!ok) {
      error = "unknown tab";
    }
  } else if (command == "page.goBack") {
    ok = GoBack(JsonString(payload, "tabId"));
    if (!ok) {
      error = "unknown tab";
    }
  } else if (command == "page.goForward") {
    ok = GoForward(JsonString(payload, "tabId"));
    if (!ok) {
      error = "unknown tab";
    }
  } else if (command == "window.create") {
    // Window creation is owned by the app controller; acknowledge only.
    ok = true;
  } else if (command == "window.close") {
    // Window close is owned by the app controller; acknowledge only.
    ok = true;
  } else if (command == "devtools.open") {
    ok = OpenDevTools();
    if (!ok) {
      error = "failed to open devtools";
    }
  } else if (command == "ui.overlay.set") {
    const bool active = JsonBool(payload, "active");
    const double width =
        payload->HasKey("width") ? payload->GetDouble("width") : 392.0;
    const double height =
        payload->HasKey("height") ? payload->GetDouble("height") : 560.0;
    ok = SetUiOverlayActive(active, width, height);
    if (!ok) {
      error = "failed to set overlay";
    }
  } else if (command == "permission.changed") {
    ok = SetPermission(JsonString(payload, "origin"),
                       JsonString(payload, "permission"),
                       JsonString(payload, "value"));
    if (!ok) {
      error = "failed to set permission";
    }
  } else {
    error = "unsupported host command: " + command;
  }

  return app_.Engine().PushHostCommandResultJson(
      HostCommandResultJson(commandId, ok, error));
}


std::string BrowserWindow::DownloadPathFor(const std::string& suggestedName) const {
  std::string directory = Store().GetSetting("downloadDirectory");
  if (directory.empty()) {
    const char* home = std::getenv("HOME");
    directory = home ? (std::filesystem::path(home) / "Downloads").string() : "/tmp";
  }
  std::filesystem::create_directories(directory);
  const std::string fileName = suggestedName.empty() ? "download" : suggestedName;
  return (std::filesystem::path(directory) / fileName).string();
}

void BrowserWindow::SetUiBrowser(CefRefPtr<CefBrowser> browser) {
  uiBrowser_ = browser;
  NSView* view = reinterpret_cast<NSView*>(browser->GetHost()->GetWindowHandle());
  MakeViewTreeTransparent(uiHostView_);
  MakeViewTreeTransparent(view);
  dispatch_async(dispatch_get_main_queue(), ^{
    MakeViewTreeTransparent(uiHostView_);
    MakeViewTreeTransparent(view);
  });
}

void BrowserWindow::OnTabBrowserCreated(const std::string& tabId, CefRefPtr<CefBrowser> browser) {
  tabManager_.SetBrowser(tabId, browser);
  if (Tab* tab = tabManager_.GetTab(tabId)) {
    try {
      tab->zoomLevel = std::stod(Store().GetSetting("defaultZoomLevel"));
    } catch (...) {
      tab->zoomLevel = 0.0;
    }
    browser->GetHost()->SetZoomLevel(tab->zoomLevel);
  }
  SetActiveContentView();
}

void BrowserWindow::OnTabTitle(const std::string& tabId, const std::string& title) {
  if (Tab* tab = tabManager_.GetTab(tabId)) {
    UpdateTabPatch(tabId, title, tab->url, tab->isLoading, tab->canGoBack, tab->canGoForward);
    PushHostEventJson(HostEventJson("page.titleChanged", {{ "tabId", JsonStringValue(tabId) }, { "title", JsonStringValue(title) }}));
  }
}

void BrowserWindow::OnTabUrl(const std::string& tabId, const std::string& url) {
  if (Tab* tab = tabManager_.GetTab(tabId)) {
    if (tab->isPendingPopup && !url.empty() && url != "about:blank") {
      tab->isPendingPopup = false;
    }
    UpdateTabPatch(tabId, tab->title, url, tab->isLoading, tab->canGoBack, tab->canGoForward);
    PushHostEventJson(HostEventJson("page.urlChanged", {{ "tabId", JsonStringValue(tabId) }, { "url", JsonStringValue(url) }}));
  }
}

void BrowserWindow::OnTabFavicon(const std::string& tabId, const std::string& faviconUrl) {
  if (Tab* tab = tabManager_.GetTab(tabId)) {
    Tab patch = *tab;
    patch.faviconUrl = faviconUrl;
    tabManager_.UpdateTab(tabId, patch);
    PushHostEventJson(HostEventJson("page.faviconChanged", {{ "tabId", JsonStringValue(tabId) }, { "faviconUrl", JsonStringValue(faviconUrl) }}));
  }
}

void BrowserWindow::OnTabLoadingState(const std::string& tabId, bool isLoading, bool canGoBack,
                                      bool canGoForward) {
  if (Tab* tab = tabManager_.GetTab(tabId)) {
    UpdateTabPatch(tabId, tab->title, tab->url, isLoading, canGoBack, canGoForward);
    PushHostEventJson(HostEventJson("page.loadingChanged",
                                    {{ "tabId", JsonStringValue(tabId) }, { "isLoading", JsonBoolValue(isLoading) }}));
    PushHostEventJson(HostEventJson("page.navigationStateChanged",
                                    {{ "tabId", JsonStringValue(tabId) },
                                     { "canGoBack", JsonBoolValue(canGoBack) },
                                     { "canGoForward", JsonBoolValue(canGoForward) }}));
  }
}

void BrowserWindow::OnNavigationStarted(const std::string& tabId) {
  if (Tab* tab = tabManager_.GetTab(tabId)) {
    eventBus_.Publish(
        {EventType::NavigationStarted, "navigation.started", *tab, windowId_, tabId, ""});
  }
}

void BrowserWindow::OnNavigationFinished(const std::string& tabId) {
  if (Tab* tab = tabManager_.GetTab(tabId)) {
    if (!privateWindow_) {
      Store().AddHistory(tab->title, tab->url, tab->faviconUrl);
      app_.PersistSession();
    }
    eventBus_.Publish(
        {EventType::NavigationFinished, "navigation.finished", *tab, windowId_, tabId, ""});
  }
}

void BrowserWindow::OnNavigationFailed(const std::string& tabId, const std::string& message) {
  if (Tab* tab = tabManager_.GetTab(tabId)) {
    Tab patch = *tab;
    patch.errorText = message;
    tabManager_.UpdateTab(tabId, patch);
    PushHostEventJson(HostEventJson("page.loadFailed", {{ "tabId", JsonStringValue(tabId) }, { "errorText", JsonStringValue(message) }}));
    if (!privateWindow_) {
      Store().AddLog("error", "Navigation failed: " + tab->url + " - " + message);
      app_.PersistSession();
    }
    eventBus_.Publish(
        {EventType::NavigationFailed, "navigation.failed", *tab, windowId_, tabId, message});
  }
}

void BrowserWindow::ExpirePendingPopupTab(const std::string& tabId) {
  if (Tab* tab = tabManager_.GetTab(tabId); tab && tab->isPendingPopup) {
    Tab patch = *tab;
    patch.isPendingPopup = false;
    tabManager_.UpdateTab(tabId, patch);
  }
}

CefWindowInfo BrowserWindow::PopupWindowInfo() const {
  return ChildWindowInfo(contentHostView_);
}

void BrowserWindow::OnDownloadStarted(const std::string& downloadId, const std::string& url,
                                      const std::string& path) {
  if (privateWindow_) {
    return;
  }
  Store().AddDownload(url, path, "started");
  Store().AddLog("info", "Download started: " + path);
  eventBus_.Publish({EventType::DownloadChanged, "download.changed", {}, windowId_, "", path});
  PageCache::Instance().Invalidate("fubuki://downloads");
}

void BrowserWindow::OnDownloadUpdated(const std::string& downloadId, const std::string& url,
                                      const std::string& path, const std::string& state,
                                      int percent) {
  if (privateWindow_) {
    return;
  }
  Store().UpdateDownload(url, path, state, percent);
  if (state != "in_progress") {
    Store().AddLog("info", "Download " + state + ": " + path);
  }
  PushHostEventJson(HostEventJson("download.updated",
                                  {{ "url", JsonStringValue(url) },
                                   { "path", JsonStringValue(path) },
                                   { "state", JsonStringValue(state) },
                                   { "percent", JsonIntValue(percent) }}));
  eventBus_.Publish({EventType::DownloadChanged, "download.changed", {}, windowId_, "", path});
}

void BrowserWindow::OnUiDraggableRegionsChanged(const std::vector<CefDraggableRegion>& regions) {
  if (!dragRegionView_ || !uiHostView_) {
    return;
  }
  if (![dragRegionView_ isKindOfClass:[FubukiDragRegionView class]]) {
    return;
  }
  [(FubukiDragRegionView*)dragRegionView_ setDraggableRegions:regions
                                                contentHeight:[uiHostView_ bounds].size.height];
}

CefRefPtr<CefDictionaryValue> BrowserWindow::SessionSnapshot() const {
  auto snapshot = CefDictionaryValue::Create();
  snapshot->SetString("id", windowId_);
  snapshot->SetBool("private", privateWindow_);
  snapshot->SetString("activeTabId", tabManager_.GetActiveTabId());
  snapshot->SetString("sidebarVisible", Store().GetSetting("sidebarVisible"));
  if (window_) {
    NSRect frame = [window_ frame];
    auto frameDict = CefDictionaryValue::Create();
    frameDict->SetDouble("x", frame.origin.x);
    frameDict->SetDouble("y", frame.origin.y);
    frameDict->SetDouble("width", frame.size.width);
    frameDict->SetDouble("height", frame.size.height);
    snapshot->SetDictionary("frame", frameDict);
  }
  auto tabs = CefListValue::Create();
  const auto tabSnapshot = tabManager_.GetTabs();
  for (size_t i = 0; i < tabSnapshot.size(); ++i) {
    auto item = CefDictionaryValue::Create();
    item->SetString("title", tabSnapshot[i].title);
    item->SetString("url", tabSnapshot[i].url.empty() ? "fubuki://newtab/" : tabSnapshot[i].url);
    item->SetString("faviconUrl", tabSnapshot[i].faviconUrl);
    item->SetBool("pinned", tabSnapshot[i].isPinned);
    item->SetBool("active", tabSnapshot[i].isActive);
    tabs->SetDictionary(i, item);
  }
  snapshot->SetList("tabs", tabs);
  return snapshot;
}

void BrowserWindow::CreateNativeWindow() {
  if (window_) {
    return;
  }
  NSRect frame = NSMakeRect(120, 120, kMinWidth, kMinHeight);
  window_ = [[NSWindow alloc]
      initWithContentRect:frame
                styleMask:NSWindowStyleMaskTitled | NSWindowStyleMaskClosable |
                          NSWindowStyleMaskMiniaturizable | NSWindowStyleMaskResizable |
                          NSWindowStyleMaskFullSizeContentView
                  backing:NSBackingStoreBuffered
                    defer:NO];
  [window_ setTitle:privateWindow_ ? @"Fubuki Browser Alpha - Private" : @"Fubuki Browser Alpha"];
  [window_ setTitleVisibility:NSWindowTitleHidden];
  [window_ setTitlebarAppearsTransparent:YES];
  [window_ setMovableByWindowBackground:YES];
  [window_ setBackgroundColor:privateWindow_ ? [NSColor colorWithCalibratedWhite:0.12 alpha:0.96]
                                             : [NSColor clearColor]];
  [window_ setOpaque:NO];
  [window_ setMinSize:NSMakeSize(kMinWidth, kMinHeight)];
  if (!gWindowDelegate) {
    gWindowDelegate = [[FubukiWindowDelegate alloc] init];
  }
  [window_ setDelegate:gWindowDelegate];
  gBrowserWindowsByNativeWindow[window_] = this;
  for (NSButton* button in @[
         [window_ standardWindowButton:NSWindowCloseButton],
         [window_ standardWindowButton:NSWindowMiniaturizeButton],
         [window_ standardWindowButton:NSWindowZoomButton],
       ]) {
    [button setHidden:NO];
    [button setAlphaValue:1.0];
  }

  NSView* root = [[NSView alloc] initWithFrame:frame];
  [root setAutoresizingMask:NSViewWidthSizable | NSViewHeightSizable];
  [root setWantsLayer:YES];
  [root.layer setBackgroundColor:[[NSColor clearColor] CGColor]];
  [window_ setContentView:root];

  uiHostView_ = [[FubukiUiHostView alloc]
      initWithFrame:NSMakeRect(0, 0, frame.size.width, frame.size.height)];
  contentHostView_ = [[NSView alloc]
      initWithFrame:NSMakeRect(kDefaultSidebarWidth, 0, frame.size.width - kDefaultSidebarWidth,
                               frame.size.height - kNavHeight)];
  dragRegionView_ = [[FubukiDragRegionView alloc] initWithFrame:[uiHostView_ frame]];
  [uiHostView_ setAutoresizingMask:NSViewWidthSizable | NSViewHeightSizable];
  [contentHostView_ setAutoresizingMask:NSViewWidthSizable | NSViewHeightSizable];
  [dragRegionView_ setAutoresizingMask:NSViewWidthSizable | NSViewHeightSizable];
  [uiHostView_ setWantsLayer:YES];
  [contentHostView_ setWantsLayer:YES];
  [uiHostView_.layer setBackgroundColor:[[NSColor clearColor] CGColor]];
  [contentHostView_.layer setBackgroundColor:[[NSColor whiteColor] CGColor]];
  [root addSubview:uiHostView_];
  [root addSubview:contentHostView_ positioned:NSWindowAbove relativeTo:uiHostView_];
  [root addSubview:dragRegionView_ positioned:NSWindowAbove relativeTo:contentHostView_];
  UpdateContentFrame();
}

void BrowserWindow::CreateUiBrowser() {
  CefBrowserSettings settings;
  settings.background_color = CefColorSetARGB(0, 255, 255, 255);
  CefBrowserHost::CreateBrowser(ChildWindowInfo(uiHostView_), new FubukiClient(this, "", true),
                                "fubuki://app/index.html?v=5", settings, nullptr, nullptr);
}

void BrowserWindow::CreateTabBrowser(const Tab& tab) {
  CefBrowserSettings settings;
  settings.background_color = CefColorSetARGB(255, 255, 255, 255);
  CefBrowserHost::CreateBrowser(ChildWindowInfo(contentHostView_),
                                new FubukiClient(this, tab.id, false), tab.url, settings, nullptr,
                                privateWindow_ ? privateRequestContext_ : nullptr);
}

bool BrowserWindow::CreateRestoredTab(CefRefPtr<CefDictionaryValue> tabState, bool active) {
  if (!tabState) {
    return false;
  }
  const std::string url =
      tabState->HasKey("url") ? tabState->GetString("url").ToString() : "fubuki://newtab/";
  const bool ok = CreateTab(url.empty() ? "fubuki://newtab/" : url, active);
  if (ok) {
    if (Tab* tab = active ? tabManager_.GetActiveTab() : nullptr) {
      const std::string restoredTitle = tabState->GetString("title").ToString();
      tab->title = restoredTitle.empty() ? tab->title : restoredTitle;
      tab->faviconUrl = tabState->GetString("faviconUrl").ToString();
      tab->isPinned = tabState->HasKey("pinned") && tabState->GetBool("pinned");
    } else {
      auto tabs = tabManager_.GetTabs();
      if (!tabs.empty()) {
        Tab* created = tabManager_.GetTab(tabs.back().id);
        if (created) {
          const std::string restoredTitle = tabState->GetString("title").ToString();
          created->title = restoredTitle.empty() ? created->title : restoredTitle;
          created->faviconUrl = tabState->GetString("faviconUrl").ToString();
          created->isPinned = tabState->HasKey("pinned") && tabState->GetBool("pinned");
        }
      }
    }
  }
  return ok;
}

void BrowserWindow::RegisterCommands() {
  auto tabIdArg = [this](CefRefPtr<CefDictionaryValue> args) -> std::string {
    return args->HasKey("tabId") ? args->GetString("tabId").ToString()
                                 : tabManager_.GetActiveTabId();
  };
  commands_.Register(
      "tabs.create", "New Tab", "Tabs", "Cmd+T", [this](CefRefPtr<CefDictionaryValue> args) {
        auto value = CefValue::Create();
        const std::string url =
            args->HasKey("url") ? args->GetString("url").ToString() : "fubuki://newtab/";
        value->SetBool(CreateTab(url, true));
        return value;
      });
  commands_.Register("tabs.close", "Close Tab", "Tabs", "Cmd+W",
                     [this, tabIdArg](CefRefPtr<CefDictionaryValue> args) {
                       auto value = CefValue::Create();
                       value->SetBool(CloseTab(tabIdArg(args)));
                       return value;
                     });
  commands_.Register("tabs.reopenClosed", "Reopen Closed Tab", "Tabs", "Cmd+Shift+T",
                     [this](CefRefPtr<CefDictionaryValue>) {
                       auto value = CefValue::Create();
                       value->SetBool(ReopenClosedTab());
                       return value;
                     });
  commands_.Register("tabs.duplicate", "Duplicate Tab", "Tabs", "",
                     [this, tabIdArg](CefRefPtr<CefDictionaryValue> args) {
                       auto value = CefValue::Create();
                       value->SetBool(DuplicateTab(tabIdArg(args)));
                       return value;
                     });
  commands_.Register("tabs.pin", "Pin Tab", "Tabs", "",
                     [this, tabIdArg](CefRefPtr<CefDictionaryValue> args) {
                       auto value = CefValue::Create();
                       value->SetBool(PinTab(tabIdArg(args), true));
                       return value;
                     });
  commands_.Register("tabs.unpin", "Unpin Tab", "Tabs", "",
                     [this, tabIdArg](CefRefPtr<CefDictionaryValue> args) {
                       auto value = CefValue::Create();
                       value->SetBool(PinTab(tabIdArg(args), false));
                       return value;
                     });
  commands_.Register("tabs.closeOther", "Close Other Tabs", "Tabs", "",
                     [this, tabIdArg](CefRefPtr<CefDictionaryValue> args) {
                       auto value = CefValue::Create();
                       value->SetBool(CloseOtherTabs(tabIdArg(args)));
                       return value;
                     });
  commands_.Register("tabs.closeToRight", "Close Tabs to the Right", "Tabs", "",
                     [this, tabIdArg](CefRefPtr<CefDictionaryValue> args) {
                       auto value = CefValue::Create();
                       value->SetBool(CloseTabsToRight(tabIdArg(args)));
                       return value;
                     });
  commands_.Register("tabs.moveToNewWindow", "Move Tab to New Window", "Tabs", "",
                     [this, tabIdArg](CefRefPtr<CefDictionaryValue> args) {
                       auto value = CefValue::Create();
                       value->SetBool(MoveTabToNewWindow(tabIdArg(args)));
                       return value;
                     });
  commands_.Register("tabs.reload", "Reload", "Navigation", "Cmd+R",
                     [this, tabIdArg](CefRefPtr<CefDictionaryValue> args) {
                       auto value = CefValue::Create();
                       value->SetBool(Reload(tabIdArg(args)));
                       return value;
                     });
  commands_.Register("tabs.stop", "Stop", "Navigation", "Esc",
                     [this, tabIdArg](CefRefPtr<CefDictionaryValue> args) {
                       auto value = CefValue::Create();
                       value->SetBool(Stop(tabIdArg(args)));
                       return value;
                     });
  commands_.Register("tabs.goBack", "Back", "Navigation", "Cmd+[",
                     [this, tabIdArg](CefRefPtr<CefDictionaryValue> args) {
                       auto value = CefValue::Create();
                       value->SetBool(GoBack(tabIdArg(args)));
                       return value;
                     });
  commands_.Register("tabs.goForward", "Forward", "Navigation", "Cmd+]",
                     [this, tabIdArg](CefRefPtr<CefDictionaryValue> args) {
                       auto value = CefValue::Create();
                       value->SetBool(GoForward(tabIdArg(args)));
                       return value;
                     });
  commands_.Register("tabs.home", "Home", "Navigation", "", [this](CefRefPtr<CefDictionaryValue>) {
    auto value = CefValue::Create();
    value->SetBool(GoHome());
    return value;
  });
  commands_.Register("tabs.activateNext", "Next Tab", "Tabs", "", [this](CefRefPtr<CefDictionaryValue>) {
    auto value = CefValue::Create();
    value->SetBool(tabManager_.ActivateNext());
    SetActiveContentView();
    return value;
  });
  commands_.Register("tabs.activatePrevious", "Previous Tab", "Tabs", "", [this](CefRefPtr<CefDictionaryValue>) {
    auto value = CefValue::Create();
    value->SetBool(tabManager_.ActivatePrevious());
    SetActiveContentView();
    return value;
  });
  commands_.Register("windows.create", "New Window", "Windows", "Cmd+N", [this](CefRefPtr<CefDictionaryValue>) {
    auto value = CefValue::Create();
    value->SetBool(app_.RequestNewWindow(false, nullptr));
    return value;
  });
  commands_.Register("windows.createPrivate", "New Private Window", "Windows", "Cmd+Shift+N", [this](CefRefPtr<CefDictionaryValue>) {
    auto value = CefValue::Create();
    value->SetBool(app_.RequestNewPrivateWindow());
    return value;
  });
  commands_.Register("windows.close", "Close Window", "Windows", "Cmd+Shift+W", [this](CefRefPtr<CefDictionaryValue>) {
    auto value = CefValue::Create();
    value->SetBool(CloseWindow());
    return value;
  });
  commands_.Register("windows.reopenClosed", "Reopen Closed Window", "Windows", "", [this](CefRefPtr<CefDictionaryValue>) {
    auto value = CefValue::Create();
    value->SetBool(app_.ReopenClosedWindow());
    return value;
  });
  commands_.Register("ui.setOverlayActive", "Set UI Overlay", "UI", "", [this](CefRefPtr<CefDictionaryValue> args) {
    auto value = CefValue::Create();
    const double overlayWidth = args->HasKey("width") ? args->GetDouble("width") : 392.0;
    const double overlayHeight = args->HasKey("height") ? args->GetDouble("height") : 560.0;
    value->SetBool(SetUiOverlayActive(args->HasKey("active") && args->GetBool("active"), overlayWidth, overlayHeight));
    return value;
  });
  commands_.Register("app.focusOmnibox", "Open Location", "Navigation", "Cmd+L", [this](CefRefPtr<CefDictionaryValue>) {
    auto value = CefValue::Create();
    value->SetBool(FocusOmnibox());
    return value;
  });
  commands_.Register("app.openSettings", "Settings", "App", "Cmd+,", [this](CefRefPtr<CefDictionaryValue>) {
    auto value = CefValue::Create();
    Tab* tab = tabManager_.GetActiveTab();
    value->SetBool(tab ? Navigate(tab->id, "fubuki://settings/") : CreateTab("fubuki://settings/", true));
    return value;
  });
  commands_.Register("app.openHistory", "History", "App", "", [this](CefRefPtr<CefDictionaryValue>) {
    auto value = CefValue::Create();
    Tab* tab = tabManager_.GetActiveTab();
    value->SetBool(tab ? Navigate(tab->id, "fubuki://history/") : CreateTab("fubuki://history/", true));
    return value;
  });
  commands_.Register("app.openDownloads", "Downloads", "App", "", [this](CefRefPtr<CefDictionaryValue>) {
    auto value = CefValue::Create();
    Tab* tab = tabManager_.GetActiveTab();
    value->SetBool(tab ? Navigate(tab->id, "fubuki://downloads/") : CreateTab("fubuki://downloads/", true));
    return value;
  });
  commands_.Register("app.openBookmarks", "Bookmarks", "App", "", [this](CefRefPtr<CefDictionaryValue>) {
    auto value = CefValue::Create();
    Tab* tab = tabManager_.GetActiveTab();
    value->SetBool(tab ? Navigate(tab->id, "fubuki://bookmarks/") : CreateTab("fubuki://bookmarks/", true));
    return value;
  });
  commands_.Register("app.openDebug", "Debug", "Developer", "", [this](CefRefPtr<CefDictionaryValue>) {
    auto value = CefValue::Create();
    Tab* tab = tabManager_.GetActiveTab();
    value->SetBool(tab ? Navigate(tab->id, "fubuki://debug/") : CreateTab("fubuki://debug/", true));
    return value;
  });
  commands_.Register("app.toggleSidebar", "Toggle Sidebar", "UI", "Cmd+B", [this](CefRefPtr<CefDictionaryValue>) {
    auto value = CefValue::Create();
    const std::string current = Store().GetSetting("sidebarVisible");
    value->SetBool(SetSetting("sidebarVisible", current == "hide" ? "show" : "hide"));
    return value;
  });
  commands_.Register("app.openDevTools", "Developer Tools", "Developer", "Cmd+Option+I", [this](CefRefPtr<CefDictionaryValue>) {
    auto value = CefValue::Create();
    value->SetBool(OpenDevTools());
    return value;
  });
  commands_.Register("page.find", "Find in Page", "Page", "Cmd+F", [this](CefRefPtr<CefDictionaryValue> args) {
    auto value = CefValue::Create();
    if (!args->HasKey("query") || args->GetString("query").empty()) {
      if (uiBrowser_) {
        uiBrowser_->GetMainFrame()->ExecuteJavaScript(
            "window.dispatchEvent(new CustomEvent('fubuki:show-find'));",
            "fubuki://app/",
            0);
        value->SetBool(true);
      } else {
        value->SetBool(false);
      }
      return value;
    }
    value->SetBool(FindInPage(args->GetString("query"), !args->HasKey("forward") || args->GetBool("forward")));
    return value;
  });
  commands_.Register("page.stopFinding", "Stop Finding", "Page", "", [this](CefRefPtr<CefDictionaryValue> args) {
    auto value = CefValue::Create();
    value->SetBool(StopFinding(!args->HasKey("clear") || args->GetBool("clear")));
    return value;
  });
  commands_.Register("page.zoomIn", "Zoom In", "Page", "Cmd++", [this](CefRefPtr<CefDictionaryValue>) {
    auto value = CefValue::Create();
    value->SetBool(ZoomIn());
    return value;
  });
  commands_.Register("page.zoomOut", "Zoom Out", "Page", "Cmd+-", [this](CefRefPtr<CefDictionaryValue>) {
    auto value = CefValue::Create();
    value->SetBool(ZoomOut());
    return value;
  });
  commands_.Register("page.zoomReset", "Actual Size", "Page", "Cmd+0", [this](CefRefPtr<CefDictionaryValue>) {
    auto value = CefValue::Create();
    value->SetBool(ResetZoom());
    return value;
  });
  commands_.Register("page.print", "Print", "Page", "Cmd+P", [this](CefRefPtr<CefDictionaryValue>) {
    auto value = CefValue::Create();
    value->SetBool(PrintPage());
    return value;
  });
  commands_.Register("page.viewSource", "View Source", "Developer", "",
                     [this](CefRefPtr<CefDictionaryValue>) {
                       auto value = CefValue::Create();
                       value->SetBool(ViewSource());
                       return value;
                     });
  commands_.Register("bookmarks.addActive", "Bookmark Active Tab", "Bookmarks", "Cmd+D",
                     [this](CefRefPtr<CefDictionaryValue>) {
                       auto value = CefValue::Create();
                       value->SetBool(AddActiveBookmark());
                       return value;
                     });
  commands_.Register("bookmarks.save", "Save Bookmark", "Bookmarks", "",
                     [this](CefRefPtr<CefDictionaryValue> args) {
                       auto value = CefValue::Create();
                       value->SetBool(SaveBookmark(args->GetString("title"), args->GetString("url"),
                                                   args->GetString("faviconUrl")));
                       return value;
                     });
  commands_.Register("bookmarks.remove", "Remove Bookmark", "Bookmarks", "",
                     [this](CefRefPtr<CefDictionaryValue> args) {
                       auto value = CefValue::Create();
                       value->SetBool(RemoveBookmark(args->GetString("url")));
                       return value;
                     });
}

void BrowserWindow::WireEvents() {
  auto emit = [this](const Event& event) {
    const bool tabEvent =
        event.type == EventType::TabCreated ||
        event.type == EventType::TabUpdated ||
        event.type == EventType::TabClosed ||
        event.type == EventType::TabActivated;
    if (tabEvent && event.windowId != windowId_) {
      return;
    }

    auto payload = CefDictionaryValue::Create();
    payload->SetString("windowId", event.windowId);
    payload->SetString("tabId", event.tabId);
    payload->SetString("message", event.message);
    if (event.type == EventType::SettingChanged) {
      payload->SetString("key", event.message);
      const std::string value = Store().GetSetting(event.message);
      payload->SetString("value", value);
    }
    if (event.type == EventType::BookmarkChanged || event.type == EventType::HistoryChanged) {
      payload->SetString("url", event.message);
    }
    if (event.type == EventType::DownloadChanged) {
      payload->SetString("path", event.message);
    }
    auto value = CefValue::Create();
    value->SetDictionary(bridge_->TabToDictionary(event.tab));
    payload->SetValue("tab", value);
    if (!tabEvent) {
      bridge_->EmitToUi(event.name, payload);
    }

    if (event.type == EventType::TabCreated) {
      bridge_->EmitToUi("tab.created", bridge_->TabToDictionary(event.tab));
    } else if (event.type == EventType::TabUpdated) {
      auto patch = bridge_->TabToDictionary(event.tab);
      patch->SetString("tabId", event.tabId);
      bridge_->EmitToUi("tab.updated", patch);
    } else if (event.type == EventType::TabClosed) {
      auto closed = CefDictionaryValue::Create();
      closed->SetString("tabId", event.tabId);
      bridge_->EmitToUi("tab.closed", closed);
    } else if (event.type == EventType::TabActivated) {
      auto activated = CefDictionaryValue::Create();
      activated->SetString("tabId", event.tabId);
      bridge_->EmitToUi("tab.activated", activated);
    } else if (event.type == EventType::NavigationStarted ||
               event.type == EventType::NavigationFinished ||
               event.type == EventType::NavigationFailed) {
    }
  };
  auto subscribe = [this, &emit](EventType type) {
    eventSubscriptions_.push_back({type, eventBus_.Subscribe(type, emit)});
  };
  subscribe(EventType::WindowCreated);
  subscribe(EventType::WindowClosed);
  subscribe(EventType::WindowFocused);
  subscribe(EventType::TabCreated);
  subscribe(EventType::TabUpdated);
  subscribe(EventType::TabClosed);
  subscribe(EventType::TabActivated);
  subscribe(EventType::NavigationStarted);
  subscribe(EventType::NavigationFinished);
  subscribe(EventType::NavigationFailed);
  subscribe(EventType::BookmarkChanged);
  subscribe(EventType::HistoryChanged);
  subscribe(EventType::DownloadChanged);
  subscribe(EventType::SettingChanged);
  subscribe(EventType::PermissionChanged);
}

void BrowserWindow::ResizeViews() {
  for (const auto& tab : tabManager_.GetTabs()) {
    if (!tab.browser) {
      continue;
    }
    NSView* view = reinterpret_cast<NSView*>(tab.browser->GetHost()->GetWindowHandle());
    [view setFrame:[contentHostView_ bounds]];
  }
}

void BrowserWindow::UpdateContentFrame() {
  if (!contentHostView_ || !uiHostView_) {
    return;
  }
  const std::string allSettingsJson = Store().GetAllSettings();
  auto settingsValue = CefParseJSON(allSettingsJson, JSON_PARSER_RFC);
  CefRefPtr<CefDictionaryValue> settings =
      (settingsValue && settingsValue->GetType() == VTYPE_DICTIONARY)
          ? settingsValue->GetDictionary()
          : CefDictionaryValue::Create();
  const std::string sidebarState = settings->GetString("sidebarVisible");
  const bool sidebarVisible = sidebarState == "show";
  double sidebarWidth = sidebarVisible ? kDefaultSidebarWidth : 0.0;
  if (sidebarVisible) {
    if (liveSidebarWidth_ > 0.0) {
      sidebarWidth = liveSidebarWidth_;
    } else {
      const std::string widthValue = settings->GetString("sidebarWidth");
      if (!widthValue.empty()) {
        try {
          sidebarWidth = std::clamp(std::stod(widthValue), static_cast<double>(kMinSidebarWidth),
                                    static_cast<double>(kMaxSidebarWidth));
        } catch (...) {
          sidebarWidth = kDefaultSidebarWidth;
        }
      }
    }
  }
  if (!sidebarVisible) {
    liveSidebarWidth_ = 0.0;
  }
  const CGFloat navHeight = kNavHeight;
  NSRect bounds = [uiHostView_ bounds];
  const CGFloat contentWidth = std::max<CGFloat>(0.0, bounds.size.width - sidebarWidth);
  const CGFloat contentHeight = std::max<CGFloat>(0.0, bounds.size.height - navHeight);
  FubukiUiHostView* uiHost = (FubukiUiHostView*)uiHostView_;
  uiHost.sidebarWidth = sidebarWidth;
  uiHost.navHeight = navHeight;
  uiHost.overlayActive = uiOverlayActive_;
  uiHost.overlayWidth = uiOverlayWidth_;
  uiHost.overlayHeight = uiOverlayHeight_;
  FubukiDragRegionView* dragHost = (FubukiDragRegionView*)dragRegionView_;
  dragHost.sidebarWidth = sidebarWidth;
  dragHost.navHeight = navHeight;
  [contentHostView_ setFrame:NSMakeRect(sidebarWidth, 0, contentWidth, contentHeight)];
  UpdateUiHostClip(uiHostView_, uiOverlayActive_, sidebarWidth, navHeight, uiOverlayWidth_,
                   uiOverlayHeight_);
  ResizeViews();
}

void BrowserWindow::UpdateTabPatch(const std::string& tabId, const std::string& title,
                                   const std::string& url, bool isLoading, bool canGoBack,
                                   bool canGoForward) {
  Tab patch;
  if (Tab* current = tabManager_.GetTab(tabId)) {
    patch = *current;
    patch.title = title;
    patch.url = url;
    patch.isLoading = isLoading;
    patch.canGoBack = canGoBack;
    patch.canGoForward = canGoForward;
    tabManager_.UpdateTab(tabId, patch);
  }
}

void BrowserWindow::SetActiveContentView() {
  for (const auto& tab : tabManager_.GetTabs()) {
    SetBrowserViewHidden(tab.browser, !tab.isActive);
    if (tab.browser && tab.isActive) {
      NSView* view = reinterpret_cast<NSView*>(tab.browser->GetHost()->GetWindowHandle());
      [contentHostView_ addSubview:view positioned:NSWindowAbove relativeTo:nil];
      [view setFrame:[contentHostView_ bounds]];
    }
  }
}

CefWindowHandle BrowserWindow::ContentParentHandle() const {
  return contentHostView_;
}

CefWindowHandle BrowserWindow::UiParentHandle() const {
  return uiHostView_;
}

}  // namespace fubuki
