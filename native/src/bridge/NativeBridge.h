#pragma once

#include <functional>
#include <string>
#include <unordered_map>

#include "commands/CommandRegistry.h"
#include "browser/Tab.h"
#include "include/cef_browser.h"
#include "include/wrapper/cef_message_router.h"
#include "include/cef_values.h"

namespace fubuki {

class BrowserWindow;

class NativeBridge : public CefMessageRouterBrowserSide::Handler {
 public:
  explicit NativeBridge(BrowserWindow& window);

  bool OnQuery(CefRefPtr<CefBrowser> browser,
               CefRefPtr<CefFrame> frame,
               int64_t query_id,
               const CefString& request,
               bool persistent,
               CefRefPtr<Callback> callback) override;
  void OnQueryCanceled(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame, int64_t query_id) override;

  CefRefPtr<CefValue> Invoke(const std::string& method, CefRefPtr<CefDictionaryValue> params);
  void EmitToUi(const std::string& eventName, CefRefPtr<CefDictionaryValue> payload);
  std::string GetStateJson() const;
  CefRefPtr<CefDictionaryValue> TabToDictionary(const Tab& tab) const;

 private:
  using MethodHandler = std::function<CefRefPtr<CefValue>(CefRefPtr<CefDictionaryValue>)>;
  void RegisterMethods();

  CefRefPtr<CefValue> ErrorValue(const std::string& message) const;
  CefRefPtr<CefValue> StateValue() const;
  std::string WriteValue(CefRefPtr<CefValue> value) const;

  BrowserWindow& window_;
  std::unordered_map<std::string, MethodHandler> methods_;
};

}  // namespace fubuki
