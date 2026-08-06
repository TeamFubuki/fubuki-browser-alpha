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
  // Host command channel fed by FrostEngine. Returns true and fills
  // |commandJson| when a pending host command is available.
  bool PollHostCommandJson(std::string &commandJson);
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
  // BrowserAppController owns the single FrostEngine instance. Every window
  // must talk to that instance; per-window engines split the source of truth
  // and cause the same mutation to be applied more than once.
  FrostBridge &frostBridge_;
  std::unordered_map<std::string, MethodHandler> methods_;
};

}  // namespace fubuki
