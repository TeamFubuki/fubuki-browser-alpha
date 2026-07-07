#include "automation/BrowserAutomation.h"

#include "browser/BrowserAppController.h"
#include "browser/BrowserWindow.h"

namespace fubuki {

namespace {

CefRefPtr<CefValue> WrapBool(bool value) {
  auto out = CefValue::Create();
  out->SetBool(value);
  return out;
}

CefRefPtr<CefDictionaryValue> TabToDict(const Tab &tab,
                                        const std::string &windowId) {
  auto item = CefDictionaryValue::Create();
  item->SetString("id", tab.id);
  item->SetString("windowId", windowId);
  item->SetString("title", tab.title);
  item->SetString("url", tab.url);
  item->SetBool("active", tab.isActive);
  item->SetBool("loading", tab.isLoading);
  item->SetBool("canGoBack", tab.canGoBack);
  item->SetBool("canGoForward", tab.canGoForward);
  return item;
}

CefRefPtr<CefValue> CopyListValue(CefRefPtr<CefListValue> list) {
  auto value = CefValue::Create();
  value->SetList(list ? list->Copy() : CefListValue::Create());
  return value;
}

}  // namespace

BrowserAutomation::BrowserAutomation(BrowserAppController &app) : app_(app) {}

CefRefPtr<CefValue> BrowserAutomation::Snapshot() {
  auto root = CefDictionaryValue::Create();
  auto windows = CefListValue::Create();
  size_t i = 0;
  for (auto *window : app_.Windows()) {
    auto w = CefDictionaryValue::Create();
    w->SetString("id", window->WindowId());
    w->SetBool("private", window->IsPrivate());
    auto tabs = CefListValue::Create();
    size_t j = 0;
    for (const auto &tab : window->Tabs().GetTabs()) {
      tabs->SetDictionary(j++, TabToDict(tab, window->WindowId()));
    }
    w->SetList("tabs", tabs);
    windows->SetDictionary(i++, w);
  }
  root->SetList("windows", windows);
  auto value = CefValue::Create();
  value->SetDictionary(root);
  return value;
}

CefRefPtr<CefValue> BrowserAutomation::ListTabs() {
  auto tabs = CefListValue::Create();
  size_t i = 0;
  for (auto *window : app_.Windows()) {
    for (const auto &tab : window->Tabs().GetTabs()) {
      tabs->SetDictionary(i++, TabToDict(tab, window->WindowId()));
    }
  }
  auto value = CefValue::Create();
  value->SetList(tabs);
  return value;
}

CefRefPtr<CefValue> BrowserAutomation::CreateTab(
    CefRefPtr<CefDictionaryValue> params) {
  auto *window = app_.ActiveWindow();
  if (!window || window->IsPrivate()) {
    return WrapBool(false);
  }
  const std::string url =
      params && params->HasKey("url") ? params->GetString("url") : "fubuki://newtab/";
  return WrapBool(window->CreateTab(url, true));
}

CefRefPtr<CefValue> BrowserAutomation::Navigate(
    CefRefPtr<CefDictionaryValue> params) {
  auto *window = app_.ActiveWindow();
  if (!window || window->IsPrivate() || !params) {
    return WrapBool(false);
  }
  return WrapBool(window->Navigate(params->GetString("tabId"),
                                   params->GetString("url")));
}

CefRefPtr<CefValue> BrowserAutomation::ActivateTab(
    CefRefPtr<CefDictionaryValue> params) {
  auto *window = app_.ActiveWindow();
  return WrapBool(window && params && !window->IsPrivate() &&
                  window->ActivateTab(params->GetString("tabId")));
}

CefRefPtr<CefValue> BrowserAutomation::CloseTab(
    CefRefPtr<CefDictionaryValue> params) {
  auto *window = app_.ActiveWindow();
  return WrapBool(window && params && !window->IsPrivate() &&
                  window->CloseTab(params->GetString("tabId")));
}

CefRefPtr<CefValue> BrowserAutomation::Reload(
    CefRefPtr<CefDictionaryValue> params) {
  auto *window = app_.ActiveWindow();
  return WrapBool(window && params && !window->IsPrivate() &&
                  window->Reload(params->GetString("tabId")));
}

CefRefPtr<CefValue> BrowserAutomation::GoBack(
    CefRefPtr<CefDictionaryValue> params) {
  auto *window = app_.ActiveWindow();
  return WrapBool(window && params && !window->IsPrivate() &&
                  window->GoBack(params->GetString("tabId")));
}

CefRefPtr<CefValue> BrowserAutomation::GoForward(
    CefRefPtr<CefDictionaryValue> params) {
  auto *window = app_.ActiveWindow();
  return WrapBool(window && params && !window->IsPrivate() &&
                  window->GoForward(params->GetString("tabId")));
}

CefRefPtr<CefValue> BrowserAutomation::ListBookmarks() {
  return CopyListValue(app_.Store().Bookmarks());
}

CefRefPtr<CefValue> BrowserAutomation::ListHistory() {
  return CopyListValue(app_.Store().History());
}

CefRefPtr<CefValue> BrowserAutomation::ListDownloads() {
  return CopyListValue(app_.Store().Downloads());
}

}  // namespace fubuki
