#pragma once

#include <functional>
#include <memory>
#include <mutex>
#include <string>
#include <unordered_map>
#include <unordered_set>

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
  NativeBridge(BrowserWindow &window, FrostBridge &frostBridge);
  ~NativeBridge() override;

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
  // Pushes a host event envelope back to FrostEngine.
  bool PushHostEventJson(const std::string &eventJson);

private:
  struct PendingQueryState {
    std::mutex mutex;
    std::unordered_set<int64_t> queryIds;
    bool accepting = true;
  };

  using MethodHandler =
      std::function<CefRefPtr<CefValue>(CefRefPtr<CefDictionaryValue>)>;
  void RegisterMethods();

  static CefRefPtr<CefValue> ErrorValue(const std::string &message);
  static CefRefPtr<CefValue> FrostResultValue(const std::string &responseJson);
  CefRefPtr<CefValue> FrostInvoke(const std::string &method,
                                  CefRefPtr<CefDictionaryValue> params);
  CefRefPtr<CefDictionaryValue> WindowToFrostDictionary(
      const BrowserWindow &window) const;
  static std::string WriteValue(CefRefPtr<CefValue> value);

  BrowserWindow &window_;
  // Owned by BrowserAppController. Every window must use this single engine
  // connection so command/result queues and logical state are global.
  FrostBridge &frostBridge_;
  std::unordered_map<std::string, MethodHandler> methods_;
  std::shared_ptr<PendingQueryState> pendingQueries_;
};

}  // namespace fubuki
