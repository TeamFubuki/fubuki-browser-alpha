#include "browser/BrowserWindow.h"

#import <Cocoa/Cocoa.h>

#include <cstdlib>
#include <filesystem>

#include "cef/FubukiClient.h"
#include "include/wrapper/cef_helpers.h"
#include "utils/UrlUtils.h"

namespace fubuki {

namespace {

constexpr CGFloat kUiHeight = 220.0;
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

}  // namespace

BrowserWindow::BrowserWindow(EventBus& eventBus, TabManager& tabManager)
    : eventBus_(eventBus),
      tabManager_(tabManager),
      bridge_(std::make_unique<NativeBridge>(*this)),
      dataStore_(std::make_unique<BrowserDataStore>(ProfilePath())) {
  dataStore_->Load();
  dataStore_->Log("info", "BrowserWindow initialized");
  RegisterCommands();
  WireEvents();
}

BrowserWindow::~BrowserWindow() = default;

void BrowserWindow::Show() {
  CEF_REQUIRE_UI_THREAD();
  CreateNativeWindow();
  CreateUiBrowser();
  CreateTab("https://example.com", true);
  [window_ makeKeyAndOrderFront:nil];
}

bool BrowserWindow::CreateTab(const std::string& input, bool active) {
  const std::string url = NormalizeNavigationInput(input);
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
    tab->browser->GetHost()->CloseBrowser(true);
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
  tab->browser->GetMainFrame()->LoadURL(NormalizeNavigationInput(input));
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
  NSView* devToolsView = [[NSView alloc] initWithFrame:[[devToolsWindow contentView] bounds]];
  [devToolsView setAutoresizingMask:NSViewWidthSizable | NSViewHeightSizable];
  [devToolsWindow setContentView:devToolsView];

  CefWindowInfo info;
  info.SetAsChild(devToolsView, CefRect(0, 0, 1000, 720));
  CefBrowserSettings settings;
  browser->GetHost()->ShowDevTools(info, new FubukiClient(this, "", false), settings, CefPoint());
  [devToolsWindow makeKeyAndOrderFront:nil];
  return true;
}

bool BrowserWindow::AddActiveBookmark() {
  Tab* tab = tabManager_.GetActiveTab();
  if (!tab) {
    return false;
  }
  const bool ok = dataStore_->AddBookmark(tab->title, tab->url);
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
  if (key != "homepage" && key != "downloadDirectory") {
    return false;
  }
  dataStore_->SetSetting(key, value);
  dataStore_->Log("info", "Setting updated: " + key);
  bridge_->EmitToUi("app.stateChanged", CefDictionaryValue::Create());
  return true;
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

void BrowserWindow::CreateNativeWindow() {
  if (window_) {
    return;
  }
  NSRect frame = NSMakeRect(120, 120, kMinWidth, kMinHeight);
  window_ = [[NSWindow alloc] initWithContentRect:frame
                                        styleMask:NSWindowStyleMaskTitled | NSWindowStyleMaskClosable |
                                                  NSWindowStyleMaskMiniaturizable | NSWindowStyleMaskResizable
                                          backing:NSBackingStoreBuffered
                                            defer:NO];
  [window_ setTitle:@"Fubuki Browser Alpha"];
  [window_ setMinSize:NSMakeSize(kMinWidth, kMinHeight)];

  NSView* root = [[NSView alloc] initWithFrame:frame];
  [root setAutoresizingMask:NSViewWidthSizable | NSViewHeightSizable];
  [window_ setContentView:root];

  uiHostView_ = [[NSView alloc] initWithFrame:NSMakeRect(0, frame.size.height - kUiHeight, frame.size.width, kUiHeight)];
  contentHostView_ = [[NSView alloc] initWithFrame:NSMakeRect(0, 0, frame.size.width, frame.size.height - kUiHeight)];
  [uiHostView_ setAutoresizingMask:NSViewWidthSizable | NSViewMinYMargin];
  [contentHostView_ setAutoresizingMask:NSViewWidthSizable | NSViewHeightSizable];
  [root addSubview:contentHostView_];
  [root addSubview:uiHostView_];
}

void BrowserWindow::CreateUiBrowser() {
  CefBrowserSettings settings;
  CefBrowserHost::CreateBrowser(ChildWindowInfo(uiHostView_),
                                new FubukiClient(this, "", true),
                                "fubuki://app/index.html?v=3",
                                settings,
                                nullptr,
                                nullptr);
}

void BrowserWindow::CreateTabBrowser(const Tab& tab) {
  CefBrowserSettings settings;
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
    bridge_->EmitToUi("app.stateChanged", CefDictionaryValue::Create());
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
