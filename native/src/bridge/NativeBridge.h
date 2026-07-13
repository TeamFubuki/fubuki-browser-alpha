#pragma once

#include <cstdint>
#include <functional>
#include <string>
#include <unordered_map>

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

  // Resolves a query that FrostEngine accepted as a pending host operation.
  // BrowserAppController must invoke this on the CEF UI thread immediately
  // after it submits the terminal HostCommandResult to FrostEngine. A failed
  // or timed out operation rejects the original renderer Promise; a success
  // returns the logical result captured when the request was accepted.
  //
  // `successResponseJson`, when supplied, must be a JSON value and replaces
  // the captured logical result. `errorCode` should be 504 for a timeout.
  bool CompletePendingOperation(const std::string &operationId, bool ok,
                                const std::string &successResponseJson = {},
                                const std::string &error = {},
                                int errorCode = 500);
  // Host command/event I/O is owned by BrowserAppController's profile-scoped
  // FrostRuntime. A bridge is only a UI endpoint and never owns an engine.

private:
  using MethodHandler =
      std::function<CefRefPtr<CefValue>(CefRefPtr<CefDictionaryValue>)>;
  void RegisterMethods();

  struct PendingQuery {
    int64_t queryId;
    int browserId;
    std::string frameId;
    CefRefPtr<Callback> callback;
    CefRefPtr<CefValue> logicalSuccessValue;
  };

  CefRefPtr<CefValue> ErrorValue(const std::string &message) const;
  CefRefPtr<CefValue> FrostResultValue(const std::string &responseJson) const;
  CefRefPtr<CefValue> FrostInvoke(const std::string &method,
                                  CefRefPtr<CefDictionaryValue> params);
  CefRefPtr<CefValue> DispatchFrostRequest(
      const std::string &method, CefRefPtr<CefDictionaryValue> params,
      const std::function<bool()> &legacyHostOperation);
  CefRefPtr<CefDictionaryValue> WindowToFrostDictionary(
      const BrowserWindow &window) const;
  std::string WriteValue(CefRefPtr<CefValue> value) const;
  bool StorePendingQuery(CefRefPtr<CefValue> response, int64_t queryId,
                         CefRefPtr<CefBrowser> browser,
                         CefRefPtr<CefFrame> frame,
                         CefRefPtr<Callback> callback);
  bool IsBridgeError(CefRefPtr<CefValue> response, std::string &error) const;
  void FailPendingQueriesForShutdown();

  BrowserWindow &window_;
  std::unordered_map<std::string, MethodHandler> methods_;
  std::unordered_map<std::string, PendingQuery> pendingQueries_;
};

}  // namespace fubuki
