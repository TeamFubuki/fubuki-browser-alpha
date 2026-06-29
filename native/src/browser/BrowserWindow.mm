#include "browser/BrowserWindow.h"

#import <Cocoa/Cocoa.h>

#include <cstdlib>
#include <filesystem>

#include "cef/FubukiClient.h"
#include "include/cef_parser.h"
#include "include/wrapper/cef_helpers.h"
#include "utils/UrlUtils.h"

@interface FubukiDragRegionView : NSView
- (void)setDraggableRegions:(const std::vector<CefDraggableRegion>&)regions contentHeight:(CGFloat)height;
@end

@interface FubukiWindowDelegate : NSObject <NSWindowDelegate>
@end

@implementation FubukiDragRegionView {
  NSMutableArray<NSValue*>* draggableRects_;
  NSMutableArray<NSValue*>* blockedRects_;
  CGFloat contentHeight_;
}

- (instancetype)initWithFrame:(NSRect)frame {
  self = [super initWithFrame:frame];
  if (self) {
    draggableRects_ = [NSMutableArray array];
    blockedRects_ = [NSMutableArray array];
    contentHeight_ = frame.size.height;
    [self setAutoresizingMask:NSViewWidthSizable | NSViewMinYMargin];
  }
  return self;
}

- (BOOL)isOpaque {
  return NO;
}

- (void)setDraggableRegions:(const std::vector<CefDraggableRegion>&)regions contentHeight:(CGFloat)height {
  [draggableRects_ removeAllObjects];
  [blockedRects_ removeAllObjects];
  contentHeight_ = height;
  for (const auto& region : regions) {
    const NSRect rect = NSMakeRect(region.bounds.x,
                                  height - region.bounds.y - region.bounds.height,
                                  region.bounds.width,
                                  region.bounds.height);
    [(region.draggable ? draggableRects_ : blockedRects_) addObject:[NSValue valueWithRect:rect]];
  }
}

- (NSView*)hitTest:(NSPoint)point {
  const NSView* hit = [super hitTest:point];
  if (hit != self) {
    return nil;
  }

  const NSPoint localPoint = point;
  if (localPoint.x >= 220.0 && localPoint.y <= contentHeight_ - 56.0) {
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
    const NSString* action = [[NSUserDefaults standardUserDefaults] stringForKey:@"AppleActionOnDoubleClick"];
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
@end

namespace fubuki {

namespace {

constexpr CGFloat kSidebarWidth = 220.0;
constexpr CGFloat kNavHeight = 56.0;
constexpr CGFloat kMinWidth = 900.0;
constexpr CGFloat kMinHeight = 620.0;

CefWindowInfo ChildWindowInfo(NSView* parent) {
  CefWindowInfo info;
  NSRect bounds = [parent bounds];
  info.SetAsChild(parent, CefRect(0, 0, static_cast<int>(bounds.size.width), static_cast<int>(bounds.size.height)));
  return info;
}

void SetBrowserViewHidden(CefRefPtr<CefBrowser> browser, bool hidden) {
  if (!browser) {
    return;
  }
  NSView* view = reinterpret_cast<NSView*>(browser->GetHost()->GetWindowHandle());
  [view setHidden:hidden];
}

std::filesystem::path ProfilePath() {
  const char* home = std::getenv("HOME");
  return home ? std::filesystem::path(home) / "Library/Application Support/Fubuki Browser Alpha"
              : std::filesystem::temp_directory_path() / "Fubuki Browser Alpha";
}

BrowserWindow* gActiveBrowserWindow = nullptr;
NSMutableArray<NSWindow*>* gDevToolsWindows = nil;
FubukiWindowDelegate* gWindowDelegate = nil;

std::string QueryParam(const std::string& url, const std::string& key) {
  const size_t queryStart = url.find('?');
  if (queryStart == std::string::npos) {
    return "";
  }
  const std::string query = url.substr(queryStart + 1);
  size_t start = 0;
  while (start <= query.size()) {
    const size_t end = query.find('&', start);
    const std::string pair = query.substr(start, end == std::string::npos ? std::string::npos : end - start);
    const size_t equals = pair.find('=');
    const std::string name = equals == std::string::npos ? pair : pair.substr(0, equals);
    if (name == key) {
      const std::string value = equals == std::string::npos ? "" : pair.substr(equals + 1);
      return CefURIDecode(value,
                          true,
                          static_cast<cef_uri_unescape_rule_t>(UU_SPACES | UU_URL_SPECIAL_CHARS_EXCEPT_PATH_SEPARATORS))
          .ToString();
    }
    if (end == std::string::npos) {
      break;
    }
    start = end + 1;
  }
  return "";
}

}  // namespace

BrowserWindow::BrowserWindow(EventBus& eventBus, TabManager& tabManager)
    : eventBus_(eventBus),
      tabManager_(tabManager),
      bridge_(std::make_unique<NativeBridge>(*this)),
      dataStore_(std::make_unique<BrowserDataStore>(ProfilePath())) {
  gActiveBrowserWindow = this;
  dataStore_->Load();
  dataStore_->Log("info", "BrowserWindow initialized");
  RegisterCommands();
  WireEvents();
}

BrowserWindow::~BrowserWindow() {
  if (gActiveBrowserWindow == this) {
    gActiveBrowserWindow = nullptr;
  }
}

bool DispatchBrowserMenuCommand(const std::string& commandId) {
  BrowserWindow* window = gActiveBrowserWindow;
  if (!window) {
    return false;
  }
  Tab* tab = window->Tabs().GetActiveTab();
  const std::string tabId = tab ? tab->id : "";
  if (commandId == "tabs.create") {
    return window->CreateTab("fubuki://newtab/", true);
  }
  if (commandId == "app.focusOmnibox") {
    return window->FocusOmnibox();
  }
  if (commandId == "app.openSettings") {
    return tab ? window->Navigate(tabId, "fubuki://settings/") : window->CreateTab("fubuki://settings/", true);
  }
  if (!tab) {
    return false;
  }
  if (commandId == "tabs.close") {
    return window->CloseTab(tabId);
  }
  if (commandId == "tabs.reload") {
    return window->Reload(tabId);
  }
  if (commandId == "tabs.stop") {
    return window->Stop(tabId);
  }
  if (commandId == "tabs.goBack") {
    return window->GoBack(tabId);
  }
  if (commandId == "tabs.goForward") {
    return window->GoForward(tabId);
  }
  return false;
}

void BrowserWindow::Show() {
  CEF_REQUIRE_UI_THREAD();
  CreateNativeWindow();
  CreateUiBrowser();
  const std::string startupBehavior = dataStore_->Settings()->GetString("startupBehavior");
  const std::string homepage = dataStore_->Settings()->GetString("homepage");
  CreateTab(startupBehavior == "newTab" ? "fubuki://newtab/" : homepage, true);
  [window_ makeKeyAndOrderFront:nil];
}

bool BrowserWindow::CreateTab(const std::string& input, bool active) {
  const std::string url = NormalizeNavigationInput(input,
                                                   dataStore_->Settings()->GetString("searchEngine"),
                                                   dataStore_->Settings()->GetString("customSearchUrl"));
  Tab& tab = tabManager_.CreateTab(url, active);
  CreateTabBrowser(tab);
  ResizeViews();
  SetActiveContentView();
  return true;
}

bool BrowserWindow::ActivateTab(const std::string& tabId) {
  const bool ok = tabManager_.ActivateTab(tabId);
  SetActiveContentView();
  return ok;
}

bool BrowserWindow::CloseTab(const std::string& tabId) {
  Tab* tab = tabManager_.GetTab(tabId);
  if (tab && tab->browser) {
    NSView* view = reinterpret_cast<NSView*>(tab->browser->GetHost()->GetWindowHandle());
    [view removeFromSuperview];
    tab->browser->GetHost()->CloseBrowser(true);
    tab->browser = nullptr;
  }
  const bool ok = tabManager_.CloseTab(tabId);
  SetActiveContentView();
  return ok;
}

bool BrowserWindow::Navigate(const std::string& tabId, const std::string& input) {
  Tab* tab = tabManager_.GetTab(tabId);
  if (!tab || !tab->browser) {
    return false;
  }
  tab->browser->GetMainFrame()->LoadURL(NormalizeNavigationInput(input,
                                                                  dataStore_->Settings()->GetString("searchEngine"),
                                                                  dataStore_->Settings()->GetString("customSearchUrl")));
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

bool BrowserWindow::FocusOmnibox() {
  if (!uiBrowser_) {
    return false;
  }
  uiBrowser_->GetHost()->SetFocus(true);
  uiBrowser_->GetMainFrame()->ExecuteJavaScript(
      "document.querySelector('.omnibox input')?.select();",
      "fubuki://app/",
      0);
  return true;
}

bool BrowserWindow::HandleShortcut(bool commandDown, bool altDown, int keyCode, char character) {
  Tab* tab = tabManager_.GetActiveTab();
  const std::string tabId = tab ? tab->id : "";
  if ((commandDown && character == 'l') || (commandDown && character == 'L')) {
    return FocusOmnibox();
  }
  if (!tab) {
    return false;
  }
  if (commandDown && (character == 'r' || character == 'R')) {
    return Reload(tabId);
  }
  if (commandDown && (character == 't' || character == 'T')) {
    return CreateTab("fubuki://newtab/", true);
  }
  if (commandDown && (character == 'w' || character == 'W')) {
    return CloseTab(tabId);
  }
  if (commandDown && (character == 'd' || character == 'D')) {
    return AddActiveBookmark();
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
  NSWindow* devToolsWindow = [[NSWindow alloc] initWithContentRect:NSMakeRect(160, 160, 1000, 720)
                                                        styleMask:NSWindowStyleMaskTitled | NSWindowStyleMaskClosable |
                                                                  NSWindowStyleMaskMiniaturizable | NSWindowStyleMaskResizable
                                                          backing:NSBackingStoreBuffered
                                                            defer:NO];
  [devToolsWindow setTitle:@"Fubuki DevTools"];
  if (!gDevToolsWindows) {
    gDevToolsWindows = [NSMutableArray array];
  }
  [gDevToolsWindows addObject:devToolsWindow];
  [devToolsWindow setReleasedWhenClosed:NO];
  NSView* devToolsView = [[NSView alloc] initWithFrame:[[devToolsWindow contentView] bounds]];
  [devToolsView setAutoresizingMask:NSViewWidthSizable | NSViewHeightSizable];
  [devToolsWindow setContentView:devToolsView];

  CefWindowInfo info;
  info.SetAsChild(devToolsView, CefRect(0, 0, 1000, 720));
  CefBrowserSettings settings;
  browser->GetHost()->ShowDevTools(info, new FubukiClient(nullptr, "", false), settings, CefPoint());
  [devToolsWindow makeKeyAndOrderFront:nil];
  return true;
}

bool BrowserWindow::AddActiveBookmark() {
  Tab* tab = tabManager_.GetActiveTab();
  if (!tab) {
    return false;
  }
  const bool ok = dataStore_->AddBookmark(tab->title, tab->url, tab->faviconUrl);
  dataStore_->Log("info", "Bookmark added: " + tab->url);
  bridge_->EmitToUi("app.stateChanged", CefDictionaryValue::Create());
  return ok;
}

bool BrowserWindow::RemoveBookmark(const std::string& url) {
  const bool ok = dataStore_->RemoveBookmark(url);
  bridge_->EmitToUi("app.stateChanged", CefDictionaryValue::Create());
  return ok;
}

bool BrowserWindow::SetSetting(const std::string& key, const std::string& value) {
  if (key != "homepage" && key != "downloadDirectory" && key != "searchEngine" && key != "startupBehavior" &&
      key != "theme" && key != "language" && key != "newTabBackgroundMode" && key != "newTabBackgroundColor" &&
      key != "newTabBackgroundUrl" && key != "customSearchUrl") {
    return false;
  }
  dataStore_->SetSetting(key, value);
  dataStore_->Log("info", "Setting updated: " + key);
  bridge_->EmitToUi("app.stateChanged", CefDictionaryValue::Create());
  return true;
}

bool BrowserWindow::SetUiOverlayActive(bool active) {
  if (!uiHostView_ || !contentHostView_) {
    return false;
  }
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
    [root addSubview:dragRegionView_ positioned:NSWindowAbove relativeTo:active ? uiHostView_ : contentHostView_];
  }
  return true;
}

bool BrowserWindow::HandleSettingsUrl(const std::string& tabId, const std::string& url) {
  const std::string key = QueryParam(url, "key");
  const std::string value = QueryParam(url, "value");
  const std::string returnPage = QueryParam(url, "return");
  bool ok = false;
  if (key == "removeBookmark") {
    ok = RemoveBookmark(value);
  } else {
    ok = SetSetting(key, value);
  }
  if (ok && tabManager_.GetTab(tabId)) {
    Navigate(tabId, returnPage.empty() ? "fubuki://settings/" : "fubuki://settings/" + returnPage);
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

std::string BrowserWindow::DownloadPathFor(const std::string& suggestedName) const {
  std::string directory = dataStore_->Settings()->GetString("downloadDirectory");
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
}

void BrowserWindow::OnTabBrowserCreated(const std::string& tabId, CefRefPtr<CefBrowser> browser) {
  tabManager_.SetBrowser(tabId, browser);
  SetActiveContentView();
}

void BrowserWindow::OnTabTitle(const std::string& tabId, const std::string& title) {
  if (Tab* tab = tabManager_.GetTab(tabId)) {
    UpdateTabPatch(tabId, title, tab->url, tab->isLoading, tab->canGoBack, tab->canGoForward);
  }
}

void BrowserWindow::OnTabUrl(const std::string& tabId, const std::string& url) {
  if (Tab* tab = tabManager_.GetTab(tabId)) {
    UpdateTabPatch(tabId, tab->title, url, tab->isLoading, tab->canGoBack, tab->canGoForward);
  }
}

void BrowserWindow::OnTabFavicon(const std::string& tabId, const std::string& faviconUrl) {
  if (Tab* tab = tabManager_.GetTab(tabId)) {
    Tab patch = *tab;
    patch.faviconUrl = faviconUrl;
    tabManager_.UpdateTab(tabId, patch);
  }
}

void BrowserWindow::OnTabLoadingState(const std::string& tabId, bool isLoading, bool canGoBack, bool canGoForward) {
  if (Tab* tab = tabManager_.GetTab(tabId)) {
    UpdateTabPatch(tabId, tab->title, tab->url, isLoading, canGoBack, canGoForward);
  }
}

void BrowserWindow::OnNavigationStarted(const std::string& tabId) {
  if (Tab* tab = tabManager_.GetTab(tabId)) {
    eventBus_.Publish({EventType::NavigationStarted, "navigation.started", *tab, tabId, ""});
  }
}

void BrowserWindow::OnNavigationFinished(const std::string& tabId) {
  if (Tab* tab = tabManager_.GetTab(tabId)) {
    dataStore_->AddHistory(tab->title, tab->url);
    eventBus_.Publish({EventType::NavigationFinished, "navigation.finished", *tab, tabId, ""});
  }
}

void BrowserWindow::OnNavigationFailed(const std::string& tabId, const std::string& message) {
  if (Tab* tab = tabManager_.GetTab(tabId)) {
    Tab patch = *tab;
    patch.errorText = message;
    tabManager_.UpdateTab(tabId, patch);
    dataStore_->Log("error", "Navigation failed: " + tab->url + " - " + message);
    eventBus_.Publish({EventType::NavigationFailed, "navigation.failed", *tab, tabId, message});
  }
}

void BrowserWindow::OnDownloadStarted(const std::string& url, const std::string& path) {
  dataStore_->AddDownload(url, path, "started");
  dataStore_->Log("info", "Download started: " + path);
  bridge_->EmitToUi("downloads.updated", CefDictionaryValue::Create());
  bridge_->EmitToUi("app.stateChanged", CefDictionaryValue::Create());
}

void BrowserWindow::OnDownloadUpdated(const std::string& url, const std::string& path, const std::string& state, int percent) {
  dataStore_->UpdateDownload(url, path, state, percent);
  if (state != "in_progress") {
    dataStore_->Log("info", "Download " + state + ": " + path);
  }
  bridge_->EmitToUi("downloads.updated", CefDictionaryValue::Create());
  bridge_->EmitToUi("app.stateChanged", CefDictionaryValue::Create());
}

void BrowserWindow::OnUiDraggableRegionsChanged(const std::vector<CefDraggableRegion>& regions) {
  if (!dragRegionView_ || !uiHostView_) {
    return;
  }
  [(FubukiDragRegionView*)dragRegionView_ setDraggableRegions:regions contentHeight:[uiHostView_ bounds].size.height];
}

void BrowserWindow::CreateNativeWindow() {
  if (window_) {
    return;
  }
  NSRect frame = NSMakeRect(120, 120, kMinWidth, kMinHeight);
  window_ = [[NSWindow alloc] initWithContentRect:frame
                                        styleMask:NSWindowStyleMaskTitled | NSWindowStyleMaskClosable |
                                                  NSWindowStyleMaskMiniaturizable | NSWindowStyleMaskResizable |
                                                  NSWindowStyleMaskFullSizeContentView
                                          backing:NSBackingStoreBuffered
                                            defer:NO];
  [window_ setTitle:@"Fubuki Browser Alpha"];
  [window_ setTitleVisibility:NSWindowTitleHidden];
  [window_ setTitlebarAppearsTransparent:YES];
  [window_ setMovableByWindowBackground:YES];
  [window_ setBackgroundColor:[NSColor clearColor]];
  [window_ setOpaque:NO];
  [window_ setMinSize:NSMakeSize(kMinWidth, kMinHeight)];
  if (!gWindowDelegate) {
    gWindowDelegate = [[FubukiWindowDelegate alloc] init];
  }
  [window_ setDelegate:gWindowDelegate];
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

  uiHostView_ = [[NSView alloc] initWithFrame:NSMakeRect(0, 0, frame.size.width, frame.size.height)];
  contentHostView_ = [[NSView alloc] initWithFrame:NSMakeRect(kSidebarWidth, 0, frame.size.width - kSidebarWidth, frame.size.height - kNavHeight)];
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
}

void BrowserWindow::CreateUiBrowser() {
  CefBrowserSettings settings;
  settings.background_color = CefColorSetARGB(0, 255, 255, 255);
  CefBrowserHost::CreateBrowser(ChildWindowInfo(uiHostView_),
                                new FubukiClient(this, "", true),
                                "fubuki://app/index.html?v=4",
                                settings,
                                nullptr,
                                nullptr);
}

void BrowserWindow::CreateTabBrowser(const Tab& tab) {
  CefBrowserSettings settings;
  settings.background_color = CefColorSetARGB(255, 255, 255, 255);
  CefBrowserHost::CreateBrowser(ChildWindowInfo(contentHostView_), new FubukiClient(this, tab.id, false), tab.url, settings, nullptr, nullptr);
}

void BrowserWindow::RegisterCommands() {
  commands_.Register("tabs.create", [this](CefRefPtr<CefDictionaryValue> args) {
    auto value = CefValue::Create();
    value->SetBool(CreateTab(args->HasKey("url") ? args->GetString("url") : "fubuki://newtab/", true));
    return value;
  });
  commands_.Register("tabs.close", [this](CefRefPtr<CefDictionaryValue> args) {
    auto value = CefValue::Create();
    value->SetBool(CloseTab(args->GetString("tabId")));
    return value;
  });
  commands_.Register("tabs.reload", [this](CefRefPtr<CefDictionaryValue> args) {
    auto value = CefValue::Create();
    value->SetBool(Reload(args->GetString("tabId")));
    return value;
  });
  commands_.Register("tabs.stop", [this](CefRefPtr<CefDictionaryValue> args) {
    auto value = CefValue::Create();
    value->SetBool(Stop(args->GetString("tabId")));
    return value;
  });
  commands_.Register("tabs.goBack", [this](CefRefPtr<CefDictionaryValue> args) {
    auto value = CefValue::Create();
    value->SetBool(GoBack(args->GetString("tabId")));
    return value;
  });
  commands_.Register("tabs.goForward", [this](CefRefPtr<CefDictionaryValue> args) {
    auto value = CefValue::Create();
    value->SetBool(GoForward(args->GetString("tabId")));
    return value;
  });
  commands_.Register("tabs.activateNext", [this](CefRefPtr<CefDictionaryValue>) {
    auto value = CefValue::Create();
    value->SetBool(tabManager_.ActivateNext());
    SetActiveContentView();
    return value;
  });
  commands_.Register("tabs.activatePrevious", [this](CefRefPtr<CefDictionaryValue>) {
    auto value = CefValue::Create();
    value->SetBool(tabManager_.ActivatePrevious());
    SetActiveContentView();
    return value;
  });
  commands_.Register("ui.setOverlayActive", [this](CefRefPtr<CefDictionaryValue> args) {
    auto value = CefValue::Create();
    value->SetBool(SetUiOverlayActive(args->HasKey("active") && args->GetBool("active")));
    return value;
  });
  commands_.Register("app.openDevTools", [this](CefRefPtr<CefDictionaryValue>) {
    auto value = CefValue::Create();
    value->SetBool(OpenDevTools());
    return value;
  });
  commands_.Register("bookmarks.addActive", [this](CefRefPtr<CefDictionaryValue>) {
    auto value = CefValue::Create();
    value->SetBool(AddActiveBookmark());
    return value;
  });
}

void BrowserWindow::WireEvents() {
  auto emit = [this](const Event& event) {
    auto payload = CefDictionaryValue::Create();
    payload->SetString("tabId", event.tabId);
    payload->SetString("message", event.message);
    auto value = CefValue::Create();
    value->SetDictionary(bridge_->TabToDictionary(event.tab));
    payload->SetValue("tab", value);
    bridge_->EmitToUi(event.name, payload);
  };
  eventBus_.Subscribe(EventType::TabCreated, emit);
  eventBus_.Subscribe(EventType::TabUpdated, emit);
  eventBus_.Subscribe(EventType::TabClosed, emit);
  eventBus_.Subscribe(EventType::TabActivated, emit);
  eventBus_.Subscribe(EventType::NavigationStarted, emit);
  eventBus_.Subscribe(EventType::NavigationFinished, emit);
  eventBus_.Subscribe(EventType::NavigationFailed, emit);
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

void BrowserWindow::UpdateTabPatch(const std::string& tabId,
                                   const std::string& title,
                                   const std::string& url,
                                   bool isLoading,
                                   bool canGoBack,
                                   bool canGoForward) {
  Tab patch;
  patch.title = title;
  patch.url = url;
  patch.isLoading = isLoading;
  patch.canGoBack = canGoBack;
  patch.canGoForward = canGoForward;
  tabManager_.UpdateTab(tabId, patch);
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
