#pragma once

#include "include/cef_values.h"

namespace fubuki {

class BrowserAppController;
struct Tab;

class PageAutomation {
public:
  explicit PageAutomation(BrowserAppController &app);

  CefRefPtr<CefValue> GetText(CefRefPtr<CefDictionaryValue> params);
  CefRefPtr<CefValue> GetHtml(CefRefPtr<CefDictionaryValue> params);
  CefRefPtr<CefValue> Screenshot(CefRefPtr<CefDictionaryValue> params);
  CefRefPtr<CefValue> GetAccessibilityTree(CefRefPtr<CefDictionaryValue> params);
  CefRefPtr<CefValue> Click(CefRefPtr<CefDictionaryValue> params);
  CefRefPtr<CefValue> Type(CefRefPtr<CefDictionaryValue> params);
  CefRefPtr<CefValue> Press(CefRefPtr<CefDictionaryValue> params);
  CefRefPtr<CefValue> Scroll(CefRefPtr<CefDictionaryValue> params);
  CefRefPtr<CefValue> Find(CefRefPtr<CefDictionaryValue> params);

private:
  Tab *TargetTab(CefRefPtr<CefDictionaryValue> params) const;
  CefRefPtr<CefValue> PageString(Tab *tab, bool html);
  CefRefPtr<CefValue> Error(const std::string &message) const;

  BrowserAppController &app_;
};

}  // namespace fubuki
