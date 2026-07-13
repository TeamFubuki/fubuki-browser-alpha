#include "bridge/NativeBridge.h"

#include "browser/BrowserAppController.h"
#include "browser/BrowserWindow.h"
#include "include/cef_parser.h"
#include "include/wrapper/cef_helpers.h"

namespace fubuki {

namespace {

bool IsAllowedUiFrame(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame,
                      CefRefPtr<CefBrowser> expectedUiBrowser) {
  // The browser-side message router can hand us a CefBrowser reference that
  // is not pointer-equal to the one retained by BrowserWindow, even though it
  // represents the same UI browser. The router is attached only to the UI
  // client, so the origin and main-frame checks are the authoritative guard.
  return browser && expectedUiBrowser && frame && frame->IsMain() &&
         frame->GetURL().ToString().rfind("fubuki://app/", 0) == 0;
}

CefRefPtr<CefValue> BoolValue(bool value) {
  auto result = CefValue::Create();
  result->SetBool(value);
  return result;
}

}  // namespace

NativeBridge::NativeBridge(BrowserWindow &window)
    : window_(window), frostBridge_(window.App().Engine()) {
  RegisterMethods();
}

void NativeBridge::RegisterMethods() {
  methods_["app.snapshot"] = [this](CefRefPtr<CefDictionaryValue>) {
    return FrostInvoke("app.snapshot", CefDictionaryValue::Create());
  };

  methods_["tabs.list"] = [this](CefRefPtr<CefDictionaryValue>) {
    return FrostInvoke("tabs.list", CefDictionaryValue::Create());
  };

  methods_["tabs.create"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return FrostInvoke("tabs.create", params);
  };

  methods_["tabs.activate"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return FrostInvoke("tabs.activate", params);
  };

  methods_["tabs.close"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return FrostInvoke("tabs.close", params);
  };

  methods_["tabs.pin"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return FrostInvoke("tabs.pin", params);
  };

  methods_["tabs.duplicate"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return FrostInvoke("tabs.duplicate", params);
  };

  methods_["tabs.reopenClosed"] = [this](CefRefPtr<CefDictionaryValue>) {
    auto params = CefDictionaryValue::Create();
    return FrostInvoke("tabs.reopenClosed", params);
  };

  methods_["tabs.closeOther"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return FrostInvoke("tabs.closeOther", params);
  };

  methods_["tabs.closeToRight"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return FrostInvoke("tabs.closeToRight", params);
  };

  methods_["tabs.move"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return FrostInvoke("tabs.move", params);
  };

  methods_["tabs.moveToNewWindow"] = [this](
                                         CefRefPtr<CefDictionaryValue> params) {
    return FrostInvoke("tabs.moveToNewWindow", params);
  };

  methods_["tabs.navigate"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return FrostInvoke("tabs.navigate", params);
  };

  methods_["tabs.reload"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return FrostInvoke("tabs.reload", params);
  };

  methods_["tabs.stop"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return FrostInvoke("tabs.stop", params);
  };

  methods_["tabs.goBack"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return FrostInvoke("tabs.goBack", params);
  };

  methods_["tabs.goForward"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return FrostInvoke("tabs.goForward", params);
  };

  methods_["tabs.home"] = [this](CefRefPtr<CefDictionaryValue>) {
    auto params = CefDictionaryValue::Create();
    return FrostInvoke("tabs.home", params);
  };

  methods_["windows.create"] = [this](CefRefPtr<CefDictionaryValue>) {
    auto params = CefDictionaryValue::Create();
    return FrostInvoke("windows.create", params);
  };

  methods_["windows.list"] = [this](CefRefPtr<CefDictionaryValue>) {
    return FrostInvoke("windows.list", CefDictionaryValue::Create());
  };

  methods_["windows.createPrivate"] = [this](CefRefPtr<CefDictionaryValue>) {
    auto params = CefDictionaryValue::Create();
    return FrostInvoke("windows.createPrivate", params);
  };

  methods_["windows.close"] = [this](CefRefPtr<CefDictionaryValue>) {
    auto params = CefDictionaryValue::Create();
    params->SetString("windowId", window_.WindowId());
    return FrostInvoke("windows.close", params);
  };

  methods_["windows.reopenClosed"] = [this](CefRefPtr<CefDictionaryValue>) {
    auto params = CefDictionaryValue::Create();
    return FrostInvoke("windows.reopenClosed", params);
  };

  methods_["page.find"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return BoolValue(window_.FindInPage(params->GetString("query"),
                                        !params->HasKey("forward") ||
                                            params->GetBool("forward")));
  };

  methods_["page.stopFinding"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return BoolValue(window_.StopFinding(!params->HasKey("clear") ||
                                         params->GetBool("clear")));
  };

  methods_["page.zoomIn"] = [this](CefRefPtr<CefDictionaryValue>) {
    return BoolValue(window_.ZoomIn());
  };

  methods_["page.zoomOut"] = [this](CefRefPtr<CefDictionaryValue>) {
    return BoolValue(window_.ZoomOut());
  };

  methods_["page.zoomReset"] = [this](CefRefPtr<CefDictionaryValue>) {
    return BoolValue(window_.ResetZoom());
  };

  methods_["page.print"] = [this](CefRefPtr<CefDictionaryValue>) {
    return BoolValue(window_.PrintPage());
  };

  methods_["page.viewSource"] = [this](CefRefPtr<CefDictionaryValue>) {
    return BoolValue(window_.ViewSource());
  };

  methods_["bookmarks.addActive"] = [this](CefRefPtr<CefDictionaryValue>) {
    return BoolValue(window_.AddActiveBookmark());
  };

  methods_["bookmarks.list"] = [this](CefRefPtr<CefDictionaryValue>) {
    return FrostInvoke("bookmarks.list", CefDictionaryValue::Create());
  };

  methods_["bookmarks.save"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return BoolValue(window_.SaveBookmark(params->GetString("title"),
                                          params->GetString("url"),
                                          params->GetString("faviconUrl")));
  };

  methods_["bookmarks.remove"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return BoolValue(window_.RemoveBookmark(params->GetString("url")));
  };

  methods_["history.remove"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return BoolValue(window_.RemoveHistory(params->GetString("url")));
  };

  methods_["history.list"] = [this](CefRefPtr<CefDictionaryValue>) {
    return FrostInvoke("history.list", CefDictionaryValue::Create());
  };

  methods_["history.clearRange"] =
      [this](CefRefPtr<CefDictionaryValue> params) {
        return BoolValue(
            window_.ClearHistoryRange(params->GetString("range")));
      };

  methods_["downloads.list"] = [this](CefRefPtr<CefDictionaryValue>) {
    return FrostInvoke("downloads.list", CefDictionaryValue::Create());
  };

  methods_["downloads.remove"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return BoolValue(window_.RemoveDownload(params->GetString("url"),
                                            params->GetString("path")));
  };

  methods_["downloads.open"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return FrostInvoke("downloads.open", params);
  };

  methods_["downloads.reveal"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return FrostInvoke("downloads.reveal", params);
  };

  methods_["data.clear"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return BoolValue(window_.ClearBrowsingData(params->GetString("target")));
  };

  methods_["settings.get"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return FrostInvoke("settings.get", params);
  };

  methods_["settings.set"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return BoolValue(window_.SetSetting(params->GetString("key"),
                                        params->GetString("value")));
  };

  methods_["settings.reset"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return BoolValue(window_.ResetSetting(params->GetString("key")));
  };

  methods_["ui.setSidebarWidth"] = [this](
                                       CefRefPtr<CefDictionaryValue> params) {
    return HostBackedFrostInvoke("ui.setSidebarWidth", params, [this, params] {
      return window_.SetLiveSidebarWidth(params->GetDouble("width"));
    });
  };

  methods_["permissions.set"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return BoolValue(window_.SetPermission(params->GetString("origin"),
                                           params->GetString("permission"),
                                           params->GetString("value")));
  };

  methods_["ui.setOverlayActive"] = [this](
                                        CefRefPtr<CefDictionaryValue> params) {
    const double overlayWidth =
        params->HasKey("width") ? params->GetDouble("width") : 392.0;
    const double overlayHeight =
        params->HasKey("height") ? params->GetDouble("height") : 560.0;
    return HostBackedFrostInvoke("ui.setOverlayActive", params,
                                 [this, params, overlayWidth, overlayHeight] {
                                   return window_.SetUiOverlayActive(
                                       params->HasKey("active") &&
                                           params->GetBool("active"),
                                       overlayWidth, overlayHeight);
                                 });
  };

  methods_["app.openDevTools"] = [this](CefRefPtr<CefDictionaryValue>) {
    return BoolValue(window_.OpenDevTools());
  };

  methods_["commands.execute"] = [this](CefRefPtr<CefDictionaryValue> params) {
    const std::string id = params->GetString("id");
    if (id.empty() || id == "commands.execute") {
      return ErrorValue("Invalid command id");
    }
    CefRefPtr<CefDictionaryValue> args = CefDictionaryValue::Create();
    if (params->HasKey("args") && params->GetType("args") == VTYPE_DICTIONARY) {
      args = params->GetDictionary("args");
    }
    if (id.rfind("tabs.", 0) == 0 && !args->HasKey("tabId")) {
      args->SetString("tabId", window_.Tabs().GetActiveTabId());
    }

    // Menu and command-palette actions must use FrostEngine as well. Directly
    // invoking BrowserWindow handlers here used to desynchronize the engine
    // from the native tab strip, especially for close and internal pages.
    if (id == "tabs.create") {
      if (!args->HasKey("active")) args->SetBool("active", true);
      if (!args->HasKey("url")) args->SetString("url", "fubuki://newtab/");
      return FrostInvoke(id, args);
    }
    if (id == "tabs.pin" || id == "tabs.unpin") {
      if (!args->HasKey("tabId") || args->GetString("tabId").empty()) {
        return BoolValue(false);
      }
      args->SetBool("pinned", id == "tabs.pin");
      return FrostInvoke("tabs.pin", args);
    }
    if (id == "tabs.close" || id == "tabs.activate" ||
        id == "tabs.duplicate" || id == "tabs.closeOther" ||
        id == "tabs.closeToRight" || id == "tabs.move" ||
        id == "tabs.moveToNewWindow" || id == "tabs.navigate" ||
        id == "tabs.reload" || id == "tabs.stop" || id == "tabs.goBack" ||
        id == "tabs.goForward") {
      if (!args->HasKey("tabId") || args->GetString("tabId").empty()) {
        return BoolValue(false);
      }
      return FrostInvoke(id, args);
    }
    if (id == "tabs.reopenClosed" || id == "tabs.home" ||
        id == "windows.create" || id == "windows.createPrivate" ||
        id == "windows.reopenClosed") {
      return FrostInvoke(id, args);
    }
    if (id == "app.openSettings" || id == "app.openHistory" ||
        id == "app.openBookmarks" || id == "app.openDownloads" ||
        id == "app.openDebug") {
      const std::string activeTabId = window_.Tabs().GetActiveTabId();
      if (activeTabId.empty()) return BoolValue(false);
      args->SetString("tabId", activeTabId);
      args->SetString("input", id == "app.openSettings"
                                    ? "fubuki://settings/"
                                : id == "app.openHistory"
                                    ? "fubuki://history/"
                                : id == "app.openBookmarks"
                                    ? "fubuki://bookmarks/"
                                : id == "app.openDownloads"
                                    ? "fubuki://downloads/"
                                    : "fubuki://debug/");
      return FrostInvoke("tabs.navigate", args);
    }
    return Invoke(id, args);
  };

  methods_["commands.list"] = [this](CefRefPtr<CefDictionaryValue>) {
    return FrostInvoke("commands.list", CefDictionaryValue::Create());
  };
}

bool NativeBridge::OnQuery(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame,
                           int64_t, const CefString &request, bool,
                           CefRefPtr<Callback> callback) {
  CEF_REQUIRE_UI_THREAD();
  if (!IsAllowedUiFrame(browser, frame, window_.UiBrowser())) {
    callback->Failure(
        403, "Native bridge is only available to the fubuki://app/ main frame");
    return true;
  }

  auto parsed = CefParseJSON(request, JSON_PARSER_RFC);
  if (!parsed || parsed->GetType() != VTYPE_DICTIONARY) {
    callback->Failure(400, "Request must be a JSON object");
    return true;
  }

  auto dict = parsed->GetDictionary();
  if (!dict->HasKey("method") || dict->GetType("method") != VTYPE_STRING) {
    callback->Failure(400, "Missing method");
    return true;
  }

  const std::string method = dict->GetString("method");
  CefRefPtr<CefDictionaryValue> params = CefDictionaryValue::Create();
  if (dict->HasKey("params") && dict->GetType("params") == VTYPE_DICTIONARY) {
    params = dict->GetDictionary("params");
  }

  auto response = Invoke(method, params);
  if (response && response->GetType() == VTYPE_DICTIONARY) {
    CefRefPtr<CefDictionaryValue> responseDictionary = response->GetDictionary();
    if (responseDictionary->HasKey("ok") &&
        !responseDictionary->GetBool("ok")) {
      callback->Failure(
          400, responseDictionary->GetString("error").ToString());
      return true;
    }
  }
  callback->Success(WriteValue(response));
  return true;
}

void NativeBridge::OnQueryCanceled(CefRefPtr<CefBrowser>, CefRefPtr<CefFrame>,
                                   int64_t) {}

CefRefPtr<CefValue> NativeBridge::Invoke(const std::string &method,
                                         CefRefPtr<CefDictionaryValue> params) {
  auto it = methods_.find(method);
  if (it != methods_.end()) {
    return it->second(params);
  }
  LOG(ERROR) << "[Bridge] Unknown method: " << method;
  return ErrorValue("Unknown bridge method: " + method);
}

CefRefPtr<CefValue>
NativeBridge::FrostResultValue(const std::string &responseJson) const {
  auto parsed = CefParseJSON(responseJson, JSON_PARSER_RFC);
  if (!parsed || parsed->GetType() != VTYPE_DICTIONARY) {
    return ErrorValue("FrostEngine returned invalid JSON");
  }

  auto response = parsed->GetDictionary();
  if (response->HasKey("ok") && !response->GetBool("ok")) {
    const std::string message =
        response->HasKey("result") ? response->GetString("result")
                                    : "FrostEngine request failed";
    return ErrorValue(message);
  }

  if (!response->HasKey("result")) {
    auto empty = CefValue::Create();
    empty->SetNull();
    return empty;
  }
  auto result = response->GetValue("result");
  return result ? result->Copy() : ErrorValue("FrostEngine response missing result");
}

CefRefPtr<CefValue>
NativeBridge::FrostInvoke(const std::string &method,
                          CefRefPtr<CefDictionaryValue> params) {
  auto request = CefDictionaryValue::Create();
  request->SetInt("version", 0);
  request->SetString("method", method);
  request->SetDictionary("params",
                         params ? params->Copy(false)
                                : CefDictionaryValue::Create());
  auto value = CefValue::Create();
  value->SetDictionary(request);
  return FrostResultValue(frostBridge_.ProcessJson(WriteValue(value)));
}

CefRefPtr<CefValue> NativeBridge::HostBackedFrostInvoke(
    const std::string &method, CefRefPtr<CefDictionaryValue> params,
    const std::function<bool()> &hostOperation) {
  const bool ok = hostOperation();
  if (ok) {
    (void)FrostInvoke(method, params);
  }
  return BoolValue(ok);
}

void NativeBridge::EmitToUi(const std::string &eventName,
                            CefRefPtr<CefDictionaryValue> payload) {
  CEF_REQUIRE_UI_THREAD();
  auto event = CefDictionaryValue::Create();
  event->SetString("name", eventName);
  event->SetDictionary("payload", payload);
  auto value = CefValue::Create();
  value->SetDictionary(event);

  const std::string script =
      "window.dispatchEvent(new CustomEvent('fubuki:event', { detail: " +
      WriteValue(value) + " }));";
  if (auto ui = window_.UiBrowser()) {
    ui->GetMainFrame()->ExecuteJavaScript(script, "fubuki://app/", 0);
  }
}

CefRefPtr<CefValue> NativeBridge::ErrorValue(const std::string &message) const {
  auto dict = CefDictionaryValue::Create();
  dict->SetBool("ok", false);
  dict->SetString("error", message);
  auto value = CefValue::Create();
  value->SetDictionary(dict);
  return value;
}

CefRefPtr<CefDictionaryValue>
NativeBridge::TabToDictionary(const Tab &tab) const {
  auto dict = CefDictionaryValue::Create();
  dict->SetString("id", tab.id);
  dict->SetString("title", tab.title);
  dict->SetString("url", tab.url);
  dict->SetString("faviconUrl", tab.faviconUrl);
  dict->SetString("errorText", tab.errorText);
  dict->SetDouble("zoomLevel", tab.zoomLevel);
  dict->SetBool("isLoading", tab.isLoading);
  dict->SetBool("canGoBack", tab.canGoBack);
  dict->SetBool("canGoForward", tab.canGoForward);
  dict->SetBool("isActive", tab.isActive);
  dict->SetBool("isPinned", tab.isPinned);
  return dict;
}

CefRefPtr<CefDictionaryValue>
NativeBridge::WindowToFrostDictionary(const BrowserWindow &window) const {
  auto dict = CefDictionaryValue::Create();
  auto tabIds = CefListValue::Create();
  const auto tabs = window.Tabs().GetTabs();
  for (size_t i = 0; i < tabs.size(); ++i) {
    tabIds->SetString(i, tabs[i].id);
  }
  dict->SetString("id", window.WindowId());
  dict->SetString("activeTabId", window.Tabs().GetActiveTabId());
  dict->SetBool("isPrivate", window.IsPrivate());
  dict->SetList("tabIds", tabIds);
  return dict;
}

std::string NativeBridge::WriteValue(CefRefPtr<CefValue> value) const {
  return CefWriteJSON(value, JSON_WRITER_DEFAULT).ToString();
}

bool NativeBridge::PushHostCommandResultJson(const std::string &resultJson) {
  return frostBridge_.PushHostCommandResultJson(resultJson);
}

bool NativeBridge::PushHostEventJson(const std::string &eventJson) {
  return frostBridge_.PushHostEventJson(eventJson);
}

}  // namespace fubuki
