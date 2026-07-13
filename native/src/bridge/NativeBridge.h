#pragma once

#include <functional>
#include <string>
#include <unordered_map>

#include "bridge/FrostBridge.h"
#include "browser/Tab.h"
#include "commands/CommandRegistry.h"
#include "include/cef_browser.h"
#include "include/cef_values.h"
#include "include/wrapper/cef_message_router.h"

namespace fubuki {

class BrowserWindow;

class NativeBridge : public CefMessageRouterBrowserSide::Handler {
public:
  explicit NativeBridge(BrowserWindow &window);

  bool OnQuery(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame,
               int64_t query_id, const CefString &request, bool persistent,
               CefRefPtr<Callback> callback) override;
  void OnQueryCanceled(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame,
                       int64_t query_id) override;

  CefRefPtr<CefValue> Invoke(const std::string &method,
                             CefRefPtr<CefDictionaryValue> params);
  void EmitToUi(const std::string &eventName,
                CefRefPtr<CefDictionaryValue> payload);
  CefRefPtr<CefDictionaryValue> TabToDictionary(const Tab &tab) const;
  // Pushes a host command result envelope back to FrostEngine.
  bool PushHostCommandResultJson(const std::string &resultJson);
  // Pushes a host event envelope back to FrostEngine.
  bool PushHostEventJson(const std::string &eventJson);

private:
  using MethodHandler =
      std::function<CefRefPtr<CefValue>(CefRefPtr<CefDictionaryValue>)>;
  void RegisterMethods();

  CefRefPtr<CefValue> ErrorValue(const std::string &message) const;
  CefRefPtr<CefValue> FrostResultValue(const std::string &responseJson) const;
  CefRefPtr<CefValue> FrostInvoke(const std::string &method,
                                  CefRefPtr<CefDictionaryValue> params);
  CefRefPtr<CefValue> HostBackedFrostInvoke(
      const std::string &method, CefRefPtr<CefDictionaryValue> params,
      const std::function<bool()> &hostOperation);
  CefRefPtr<CefDictionaryValue> WindowToFrostDictionary(
      const BrowserWindow &window) const;
  std::string WriteValue(CefRefPtr<CefValue> value) const;

  BrowserWindow &window_;
  // The application controller is the sole owner of FrostEngine. A bridge is
  // window-scoped only because CEF message-router handlers are window-scoped.
  FrostBridge &frostBridge_;
  std::unordered_map<std::string, MethodHandler> methods_;
};

}  // namespace fubuki
