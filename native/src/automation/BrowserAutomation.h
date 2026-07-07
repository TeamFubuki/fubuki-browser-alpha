#pragma once

#include <string>

#include "include/cef_values.h"

namespace fubuki {

class BrowserAppController;

class BrowserAutomation {
public:
  explicit BrowserAutomation(BrowserAppController &app);

  CefRefPtr<CefValue> Snapshot();
  CefRefPtr<CefValue> ListTabs();
  CefRefPtr<CefValue> CreateTab(CefRefPtr<CefDictionaryValue> params);
  CefRefPtr<CefValue> Navigate(CefRefPtr<CefDictionaryValue> params);
  CefRefPtr<CefValue> ActivateTab(CefRefPtr<CefDictionaryValue> params);
  CefRefPtr<CefValue> CloseTab(CefRefPtr<CefDictionaryValue> params);
  CefRefPtr<CefValue> Reload(CefRefPtr<CefDictionaryValue> params);
  CefRefPtr<CefValue> GoBack(CefRefPtr<CefDictionaryValue> params);
  CefRefPtr<CefValue> GoForward(CefRefPtr<CefDictionaryValue> params);
  CefRefPtr<CefValue> ListBookmarks();
  CefRefPtr<CefValue> ListHistory();
  CefRefPtr<CefValue> ListDownloads();

private:
  BrowserAppController &app_;
};

}  // namespace fubuki
