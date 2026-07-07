#include "browser/BrowserAppController.h"

#include <algorithm>

#include "browser/BrowserWindow.h"
#include "include/base/cef_callback.h"
#include "include/cef_parser.h"
#include "include/wrapper/cef_closure_task.h"
#include "include/wrapper/cef_helpers.h"

namespace fubuki {

namespace {

BrowserAppController *gController = nullptr;

CefRefPtr<CefValue>
ValueFromDictionary(CefRefPtr<CefDictionaryValue> dictionary) {
  auto value = CefValue::Create();
  value->SetDictionary(dictionary);
  return value;
}

}  // namespace

BrowserAppController::BrowserAppController(std::filesystem::path profilePath)
    : profilePath_(std::move(profilePath)), store_(profilePath_),
      automation_(*this) {
  store_.Load();
  automation_.RefreshFromSettings();
  store_.Log("info", "BrowserAppController initialized");
}

BrowserAppController::~BrowserAppController() = default;

void BrowserAppController::Start() {
  CEF_REQUIRE_UI_THREAD();
  const std::string startupBehavior =
      store_.Settings()->GetString("startupBehavior");
  bool restored = false;
  if (startupBehavior == "restore") {
    restoring_ = true;
    auto windows = RestoredWindows();
    for (size_t i = 0; i < windows->GetSize(); ++i) {
      if (auto windowState = windows->GetDictionary(i)) {
        NewWindow(false, windowState);
        restored = true;
      }
    }
    restoring_ = false;
  }
  if (!restored) {
    NewWindow(false, nullptr);
  }
}

BrowserWindow *
BrowserAppController::NewWindow(bool privateWindow,
                                CefRefPtr<CefDictionaryValue> restoreState) {
  CEF_REQUIRE_UI_THREAD();
  auto context = std::make_unique<WindowContext>();
  context->tabManager = std::make_unique<TabManager>(eventBus_);
  BrowserWindow *raw = nullptr;
  const std::string windowId = NextWindowId();
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
  CefPostTask(TID_UI, base::BindOnce(
                          [](BrowserAppController *app, bool privateWindow,
                             CefRefPtr<CefDictionaryValue> restoreState) {
                            if (app) {
                              app->NewWindow(privateWindow, restoreState);
                            }
                          },
                          this, privateWindow, restoreState));
  return true;
}

bool BrowserAppController::RequestNewPrivateWindow() {
  return RequestNewWindow(true, nullptr);
}

bool BrowserAppController::CloseActiveWindow() {
  if (!activeWindow_) {
    return false;
  }
  return activeWindow_->CloseWindow();
}

bool BrowserAppController::ReopenClosedWindow() {
  if (closedWindows_.empty()) {
    return false;
  }
  auto state = closedWindows_.back();
  closedWindows_.pop_back();
  NewWindow(false, state);
  return true;
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
  auto root = CefDictionaryValue::Create();
  auto windows = CefListValue::Create();
  size_t index = 0;
  for (const auto &context : windows_) {
    if (!context->window || context->window->IsPrivate()) {
      continue;
    }
    windows->SetDictionary(index++, context->window->SessionSnapshot());
  }
  root->SetInt("version", 1);
  root->SetList("windows", windows);
  store_.SetSetting(
      "sessionJson",
      CefWriteJSON(ValueFromDictionary(root), JSON_WRITER_DEFAULT).ToString());
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

std::string BrowserAppController::NextWindowId() {
  return "window-" + std::to_string(nextWindowId_++);
}

CefRefPtr<CefListValue> BrowserAppController::RestoredWindows() const {
  auto empty = CefListValue::Create();
  const std::string json = store_.Settings()->GetString("sessionJson");
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
    window = app->NewWindow(false, nullptr);
  }
  if (!window) {
    return false;
  }
  auto result = window->ExecuteCommand(commandId, CefDictionaryValue::Create());
  if (!result || result->GetType() == VTYPE_NULL) {
    return false;
  }
  return result->GetType() == VTYPE_BOOL ? result->GetBool() : true;
}

}  // namespace fubuki
