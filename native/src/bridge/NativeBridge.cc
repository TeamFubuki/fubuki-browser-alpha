#include "bridge/NativeBridge.h"

#include "browser/BrowserAppController.h"
#include "browser/BrowserWindow.h"
#include "include/cef_parser.h"
#include "include/wrapper/cef_helpers.h"

namespace fubuki {

namespace {

bool IsAllowedUiFrame(CefRefPtr<CefFrame> frame) {
  return frame && frame->GetURL().ToString().rfind("fubuki://app/", 0) == 0;
}

CefRefPtr<CefValue> BoolValue(bool value) {
  auto result = CefValue::Create();
  result->SetBool(value);
  return result;
}

}  // namespace

NativeBridge::NativeBridge(BrowserWindow &window)
    : window_(window),
      frostBridge_(window.Store().ProfilePath() + "/frost-engine.sqlite3") {
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
    // Tabs are created by FrostEngine. It emits one page.create HostCommand,
    // which BrowserAppController dispatches to the native window. Creating a
    // native tab here as well produces a duplicate tab with a different ID.
    return FrostInvoke("tabs.create", params);
  };

  methods_["tabs.activate"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return HostBackedFrostInvoke("tabs.activate", params, [this, params] {
      return window_.ActivateTab(params->GetString("tabId"));
    });
  };

  methods_["tabs.close"] = [this](CefRefPtr<CefDictionaryValue> params) {
    // FrostEngine owns the close lifecycle and emits page.close for the host
    // to execute. Closing natively first leaves that delayed command pointing
    // at a tab that no longer exists.
    return FrostInvoke("tabs.close", params);
  };

  methods_["tabs.pin"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return HostBackedFrostInvoke("tabs.pin", params, [this, params] {
      return window_.PinTab(params->GetString("tabId"),
                            params->HasKey("pinned") &&
                                params->GetBool("pinned"));
    });
  };

  methods_["tabs.duplicate"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return HostBackedFrostInvoke("tabs.duplicate", params, [this, params] {
      return window_.DuplicateTab(params->GetString("tabId"));
    });
  };

  methods_["tabs.reopenClosed"] = [this](CefRefPtr<CefDictionaryValue>) {
    auto params = CefDictionaryValue::Create();
    return HostBackedFrostInvoke("tabs.reopenClosed", params,
                                 [this] { return window_.ReopenClosedTab(); });
  };

  methods_["tabs.closeOther"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return HostBackedFrostInvoke("tabs.closeOther", params, [this, params] {
      return window_.CloseOtherTabs(params->GetString("tabId"));
    });
  };

  methods_["tabs.closeToRight"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return HostBackedFrostInvoke("tabs.closeToRight", params, [this, params] {
      return window_.CloseTabsToRight(params->GetString("tabId"));
    });
  };

  methods_["tabs.move"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return HostBackedFrostInvoke("tabs.move", params, [this, params] {
      return window_.MoveTab(params->GetString("tabId"),
                             params->GetInt("toIndex"));
    });
  };

  methods_["tabs.moveToNewWindow"] = [this](
                                         CefRefPtr<CefDictionaryValue> params) {
    return HostBackedFrostInvoke("tabs.moveToNewWindow", params,
                                 [this, params] {
                                   return window_.MoveTabToNewWindow(
                                       params->GetString("tabId"));
                                 });
  };

  methods_["tabs.navigate"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return HostBackedFrostInvoke("tabs.navigate", params, [this, params] {
      return window_.Navigate(params->GetString("tabId"),
                              params->GetString("input"));
    });
  };

  methods_["tabs.reload"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return HostBackedFrostInvoke("tabs.reload", params, [this, params] {
      return window_.Reload(params->GetString("tabId"));
    });
  };

  methods_["tabs.stop"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return HostBackedFrostInvoke("tabs.stop", params, [this, params] {
      return window_.Stop(params->GetString("tabId"));
    });
  };

  methods_["tabs.goBack"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return HostBackedFrostInvoke("tabs.goBack", params, [this, params] {
      return window_.GoBack(params->GetString("tabId"));
    });
  };

  methods_["tabs.goForward"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return HostBackedFrostInvoke("tabs.goForward", params, [this, params] {
      return window_.GoForward(params->GetString("tabId"));
    });
  };

  methods_["tabs.home"] = [this](CefRefPtr<CefDictionaryValue>) {
    auto params = CefDictionaryValue::Create();
    return HostBackedFrostInvoke("tabs.home", params,
                                 [this] { return window_.GoHome(); });
  };

  methods_["windows.create"] = [this](CefRefPtr<CefDictionaryValue>) {
    auto params = CefDictionaryValue::Create();
    return HostBackedFrostInvoke("windows.create", params, [this] {
      return window_.App().RequestNewWindow(false, nullptr);
    });
  };

  methods_["windows.list"] = [this](CefRefPtr<CefDictionaryValue>) {
    return FrostInvoke("windows.list", CefDictionaryValue::Create());
  };

  methods_["windows.createPrivate"] = [this](CefRefPtr<CefDictionaryValue>) {
    auto params = CefDictionaryValue::Create();
    return HostBackedFrostInvoke("windows.createPrivate", params, [this] {
      return window_.App().RequestNewPrivateWindow();
    });
  };

  methods_["windows.close"] = [this](CefRefPtr<CefDictionaryValue>) {
    auto params = CefDictionaryValue::Create();
    params->SetString("windowId", window_.WindowId());
    return HostBackedFrostInvoke("windows.close", params,
                                 [this] { return window_.CloseWindow(); });
  };

  methods_["windows.reopenClosed"] = [this](CefRefPtr<CefDictionaryValue>) {
    auto params = CefDictionaryValue::Create();
    return HostBackedFrostInvoke("windows.reopenClosed", params, [this] {
      return window_.App().ReopenClosedWindow();
    });
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
    return HostBackedFrostInvoke("bookmarks.save", params, [this, params] {
      return window_.SaveBookmark(params->GetString("title"),
                                  params->GetString("url"),
                                  params->GetString("faviconUrl"));
    });
  };

  methods_["bookmarks.remove"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return HostBackedFrostInvoke("bookmarks.remove", params, [this, params] {
      return window_.RemoveBookmark(params->GetString("url"));
    });
  };

  methods_["history.remove"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return HostBackedFrostInvoke("history.remove", params, [this, params] {
      return window_.RemoveHistory(params->GetString("url"));
    });
  };

  methods_["history.list"] = [this](CefRefPtr<CefDictionaryValue>) {
    return FrostInvoke("history.list", CefDictionaryValue::Create());
  };

  methods_["history.clearRange"] =
      [this](CefRefPtr<CefDictionaryValue> params) {
        return HostBackedFrostInvoke("history.clearRange", params,
                                     [this, params] {
                                       return window_.ClearHistoryRange(
                                           params->GetString("range"));
                                     });
      };

  methods_["downloads.list"] = [this](CefRefPtr<CefDictionaryValue>) {
    return FrostInvoke("downloads.list", CefDictionaryValue::Create());
  };

  methods_["downloads.remove"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return HostBackedFrostInvoke("downloads.remove", params, [this, params] {
      return window_.RemoveDownload(params->GetString("url"),
                                    params->GetString("path"));
    });
  };

  methods_["downloads.open"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return BoolValue(window_.OpenDownloadedFile(params->GetString("path")));
  };

  methods_["downloads.reveal"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return BoolValue(window_.RevealDownloadedFile(params->GetString("path")));
  };

  methods_["data.clear"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return HostBackedFrostInvoke("data.clear", params, [this, params] {
      return window_.ClearBrowsingData(params->GetString("target"));
    });
  };

  methods_["settings.get"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return FrostInvoke("settings.get", params);
  };

  methods_["settings.set"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return HostBackedFrostInvoke("settings.set", params, [this, params] {
      return window_.SetSetting(params->GetString("key"),
                                params->GetString("value"));
    });
  };

  methods_["settings.reset"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return HostBackedFrostInvoke("settings.reset", params, [this, params] {
      return window_.ResetSetting(params->GetString("key"));
    });
  };

  methods_["ui.setSidebarWidth"] = [this](
                                       CefRefPtr<CefDictionaryValue> params) {
    return HostBackedFrostInvoke("ui.setSidebarWidth", params, [this, params] {
      return window_.SetLiveSidebarWidth(params->GetDouble("width"));
    });
  };

  methods_["permissions.set"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return HostBackedFrostInvoke("permissions.set", params, [this, params] {
      return window_.SetPermission(params->GetString("origin"),
                                   params->GetString("permission"),
                                   params->GetString("value"));
    });
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
    CefRefPtr<CefDictionaryValue> args = CefDictionaryValue::Create();
    if (params->HasKey("args") && params->GetType("args") == VTYPE_DICTIONARY) {
      args = params->GetDictionary("args");
    }
    auto result = window_.Commands().Execute(id, args);
    (void)FrostInvoke("commands.execute", params);
    return result;
  };

  methods_["commands.list"] = [this](CefRefPtr<CefDictionaryValue>) {
    return FrostInvoke("commands.list", CefDictionaryValue::Create());
  };
}

bool NativeBridge::OnQuery(CefRefPtr<CefBrowser>, CefRefPtr<CefFrame> frame,
                           int64_t, const CefString &request, bool,
                           CefRefPtr<Callback> callback) {
  CEF_REQUIRE_UI_THREAD();
  if (!IsAllowedUiFrame(frame)) {
    callback->Failure(403, "Native bridge is only available to fubuki://app/");
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

bool NativeBridge::PollHostCommandJson(std::string &commandJson) {
  return frostBridge_.PollHostCommandJson(commandJson);
}

bool NativeBridge::PushHostCommandResultJson(const std::string &resultJson) {
  return frostBridge_.PushHostCommandResultJson(resultJson);
}

bool NativeBridge::PushHostEventJson(const std::string &eventJson) {
  return frostBridge_.PushHostEventJson(eventJson);
}

}  // namespace fubuki
