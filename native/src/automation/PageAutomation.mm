#include "automation/PageAutomation.h"

#import <Cocoa/Cocoa.h>

#include <condition_variable>
#include <mutex>

#include "browser/BrowserAppController.h"
#include "browser/BrowserWindow.h"
#include "include/cef_browser.h"
#include "include/cef_frame.h"
#include "include/cef_string_visitor.h"

namespace fubuki {

namespace {

bool IsInternalUrl(const std::string &url) {
  return url.rfind("fubuki://", 0) == 0;
}

CefRefPtr<CefValue> BoolValue(bool ok) {
  auto value = CefValue::Create();
  value->SetBool(ok);
  return value;
}

class BlockingStringVisitor : public CefStringVisitor {
public:
  void Visit(const CefString &string) override {
    {
      std::lock_guard<std::mutex> lock(mutex_);
      value_ = string.ToString();
      done_ = true;
    }
    cv_.notify_one();
  }

  std::string Wait() {
    std::unique_lock<std::mutex> lock(mutex_);
    cv_.wait_for(lock, std::chrono::seconds(2), [this] { return done_; });
    return value_;
  }

private:
  std::mutex mutex_;
  std::condition_variable cv_;
  std::string value_;
  bool done_ = false;

  IMPLEMENT_REFCOUNTING(BlockingStringVisitor);
};

}  // namespace

PageAutomation::PageAutomation(BrowserAppController &app) : app_(app) {}

Tab *PageAutomation::TargetTab(CefRefPtr<CefDictionaryValue> params) const {
  auto *window = app_.ActiveWindow();
  if (!window || window->IsPrivate()) {
    return nullptr;
  }
  if (params && params->HasKey("tabId")) {
    return window->Tabs().GetTab(params->GetString("tabId"));
  }
  return window->Tabs().GetActiveTab();
}

CefRefPtr<CefValue> PageAutomation::Error(const std::string &message) const {
  auto dict = CefDictionaryValue::Create();
  dict->SetString("error", message);
  auto value = CefValue::Create();
  value->SetDictionary(dict);
  return value;
}

CefRefPtr<CefValue> PageAutomation::PageString(Tab *tab, bool html) {
  if (!tab || !tab->browser || IsInternalUrl(tab->url)) {
    return Error("Page automation is unavailable for this tab");
  }
  CefRefPtr<CefFrame> frame = tab->browser->GetMainFrame();
  if (!frame) {
    return Error("No main frame");
  }
  CefRefPtr<BlockingStringVisitor> visitor = new BlockingStringVisitor();
  if (html) {
    frame->GetSource(visitor);
  } else {
    frame->GetText(visitor);
  }
  auto value = CefValue::Create();
  value->SetString(visitor->Wait());
  return value;
}

CefRefPtr<CefValue> PageAutomation::GetText(
    CefRefPtr<CefDictionaryValue> params) {
  return PageString(TargetTab(params), false);
}

CefRefPtr<CefValue> PageAutomation::GetHtml(
    CefRefPtr<CefDictionaryValue> params) {
  return PageString(TargetTab(params), true);
}

CefRefPtr<CefValue> PageAutomation::Screenshot(
    CefRefPtr<CefDictionaryValue> params) {
  auto *tab = TargetTab(params);
  if (!tab || !tab->browser || IsInternalUrl(tab->url)) {
    return Error("Screenshot is unavailable for this tab");
  }
  CefWindowHandle handle = tab->browser->GetHost()->GetWindowHandle();
  NSView *view = reinterpret_cast<NSView *>(handle);
  if (!view) {
    return Error("No native view for tab");
  }
  NSRect bounds = [view bounds];
  NSBitmapImageRep *rep = [view bitmapImageRepForCachingDisplayInRect:bounds];
  if (!rep) {
    return Error("Unable to allocate screenshot buffer");
  }
  [view cacheDisplayInRect:bounds toBitmapImageRep:rep];
  NSData *data = [rep representationUsingType:NSBitmapImageFileTypePNG
                                   properties:@{}];
  if (!data) {
    return Error("Unable to encode screenshot");
  }
  NSString *base64 = [data base64EncodedStringWithOptions:0];
  auto dict = CefDictionaryValue::Create();
  dict->SetString("mimeType", "image/png");
  dict->SetString("base64", [base64 UTF8String]);
  auto value = CefValue::Create();
  value->SetDictionary(dict);
  return value;
}

CefRefPtr<CefValue> PageAutomation::GetAccessibilityTree(
    CefRefPtr<CefDictionaryValue> params) {
  auto *tab = TargetTab(params);
  if (!tab || IsInternalUrl(tab->url)) {
    return Error("Accessibility tree is unavailable for this tab");
  }
  auto root = CefDictionaryValue::Create();
  root->SetString("role", "document");
  root->SetString("name", tab->title);
  root->SetString("url", tab->url);
  auto value = CefValue::Create();
  value->SetDictionary(root);
  return value;
}

CefRefPtr<CefValue> PageAutomation::Click(CefRefPtr<CefDictionaryValue> params) {
  auto *tab = TargetTab(params);
  if (!tab || !tab->browser || IsInternalUrl(tab->url) || !params) {
    return BoolValue(false);
  }
  CefMouseEvent event;
  event.x = params->HasKey("x") ? params->GetInt("x") : 0;
  event.y = params->HasKey("y") ? params->GetInt("y") : 0;
  tab->browser->GetHost()->SendMouseClickEvent(event, MBT_LEFT, false, 1);
  tab->browser->GetHost()->SendMouseClickEvent(event, MBT_LEFT, true, 1);
  return BoolValue(true);
}

CefRefPtr<CefValue> PageAutomation::Type(CefRefPtr<CefDictionaryValue> params) {
  auto *tab = TargetTab(params);
  if (!tab || !tab->browser || IsInternalUrl(tab->url) || !params) {
    return BoolValue(false);
  }
  const std::string text = params->GetString("text");
  for (const char c : text) {
    CefKeyEvent event;
    event.type = KEYEVENT_CHAR;
    event.character = static_cast<char16_t>(c);
    event.unmodified_character = static_cast<char16_t>(c);
    tab->browser->GetHost()->SendKeyEvent(event);
  }
  return BoolValue(true);
}

CefRefPtr<CefValue> PageAutomation::Press(CefRefPtr<CefDictionaryValue> params) {
  auto *tab = TargetTab(params);
  if (!tab || !tab->browser || IsInternalUrl(tab->url) || !params) {
    return BoolValue(false);
  }
  const std::string key = params->GetString("key");
  if (key.empty()) {
    return BoolValue(false);
  }
  CefKeyEvent down;
  down.type = KEYEVENT_RAWKEYDOWN;
  down.windows_key_code = key[0];
  CefKeyEvent up = down;
  up.type = KEYEVENT_KEYUP;
  tab->browser->GetHost()->SendKeyEvent(down);
  tab->browser->GetHost()->SendKeyEvent(up);
  return BoolValue(true);
}

CefRefPtr<CefValue> PageAutomation::Scroll(
    CefRefPtr<CefDictionaryValue> params) {
  auto *tab = TargetTab(params);
  if (!tab || !tab->browser || IsInternalUrl(tab->url) || !params) {
    return BoolValue(false);
  }
  CefMouseEvent event;
  event.x = params->HasKey("x") ? params->GetInt("x") : 0;
  event.y = params->HasKey("y") ? params->GetInt("y") : 0;
  const int deltaX = params->HasKey("deltaX") ? params->GetInt("deltaX") : 0;
  const int deltaY = params->HasKey("deltaY") ? params->GetInt("deltaY") : -120;
  tab->browser->GetHost()->SendMouseWheelEvent(event, deltaX, deltaY);
  return BoolValue(true);
}

CefRefPtr<CefValue> PageAutomation::Find(CefRefPtr<CefDictionaryValue> params) {
  auto *window = app_.ActiveWindow();
  if (!window || window->IsPrivate() || !params) {
    return BoolValue(false);
  }
  return BoolValue(window->FindInPage(params->GetString("query"), true));
}

}  // namespace fubuki
