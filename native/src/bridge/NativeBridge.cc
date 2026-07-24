#include "bridge/NativeBridge.h"

#include <algorithm>

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

double NumberValue(CefRefPtr<CefDictionaryValue> dictionary,
                   const std::string &key, double fallback = 0.0) {
  if (!dictionary || !dictionary->HasKey(key)) {
    return fallback;
  }
  const cef_value_type_t type = dictionary->GetType(key);
  if (type == VTYPE_INT) {
    return static_cast<double>(dictionary->GetInt(key));
  }
  if (type == VTYPE_DOUBLE) {
    return dictionary->GetDouble(key);
  }
  return fallback;
}

}  // namespace

NativeBridge::NativeBridge(BrowserWindow &window)
    : window_(window),
      frostBridge_(window.App().Engine()) {
  RegisterMethods();
}

void NativeBridge::RegisterMethods() {
  methods_["app.snapshot"] = [this](CefRefPtr<CefDictionaryValue>) {
    // FrostEngine owns global activeWindowId; this bridge must not rewrite it.
    auto result = FrostInvoke("app.snapshot", CefDictionaryValue::Create());
    if (result && result->GetType() == VTYPE_DICTIONARY) {
      auto snapshot = result->GetDictionary();
      // activeWindowId is global; every BrowserWindow needs its own local
      // rendering context to select tabs from the shared snapshot.
      snapshot->SetString("currentWindowId", window_.WindowId());
    }
    return result;
  };

  methods_["tabs.list"] = [this](CefRefPtr<CefDictionaryValue>) {
    return FrostInvoke("tabs.list", CefDictionaryValue::Create());
  };

  methods_["tabs.create"] = [this](CefRefPtr<CefDictionaryValue> params) {
    if (!params) params = CefDictionaryValue::Create();
    params->SetString("windowId", window_.WindowId());
    // Native shortcuts and menu commands do not carry UI defaults. The
    // protocol requires `active`, so make a newly created shortcut tab active
    // just like the UI-side tabs.create helper does.
    if (!params->HasKey("active")) {
      params->SetBool("active", true);
    }
    return FrostInvoke("tabs.create", params);
  };

  methods_["tabs.activate"] = [this](CefRefPtr<CefDictionaryValue> params) {
    auto result = FrostInvoke("tabs.activate", params);
    // FrostEngine emits page.activate asynchronously. Keep the native tab
    // manager in sync immediately as well: a following Cmd+W is handled by
    // the native key path and must use the tab the user just selected.
    if (result && result->GetType() == VTYPE_BOOL && result->GetBool() &&
        params && params->HasKey("tabId")) {
      window_.ActivateTab(params->GetString("tabId").ToString());
    }
    return result;
  };

  // The engine's global active tab belongs to whichever native window was
  // focused last. Resolve next/previous from this window's TabManager first,
  // then activate that explicit tab in FrostEngine. This keeps tab cycling
  // scoped to the window that received the shortcut.
  methods_["tabs.activateNext"] = [this](CefRefPtr<CefDictionaryValue>) {
    const auto tabs = window_.Tabs().GetTabs();
    const std::string activeId = window_.Tabs().GetActiveTabId();
    if (tabs.empty() || activeId.empty()) {
      return BoolValue(false);
    }
    const auto current = std::find_if(
        tabs.begin(), tabs.end(), [&activeId](const Tab& tab) { return tab.id == activeId; });
    if (current == tabs.end()) {
      return BoolValue(false);
    }
    const size_t index = static_cast<size_t>(std::distance(tabs.begin(), current));
    auto params = CefDictionaryValue::Create();
    params->SetString("tabId", tabs[(index + 1) % tabs.size()].id);
    return FrostInvoke("tabs.activate", params);
  };

  methods_["tabs.activatePrevious"] = [this](CefRefPtr<CefDictionaryValue>) {
    const auto tabs = window_.Tabs().GetTabs();
    const std::string activeId = window_.Tabs().GetActiveTabId();
    if (tabs.empty() || activeId.empty()) {
      return BoolValue(false);
    }
    const auto current = std::find_if(
        tabs.begin(), tabs.end(), [&activeId](const Tab& tab) { return tab.id == activeId; });
    if (current == tabs.end()) {
      return BoolValue(false);
    }
    const size_t index = static_cast<size_t>(std::distance(tabs.begin(), current));
    auto params = CefDictionaryValue::Create();
    params->SetString("tabId", tabs[(index + tabs.size() - 1) % tabs.size()].id);
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
    if (auto *tab = window_.Tabs().GetActiveTab()) {
      params->SetString("tabId", tab->id);
    }
    params->SetString("windowId", window_.WindowId());
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
    return FrostInvoke("windows.reopenClosed", CefDictionaryValue::Create());
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
    auto *tab = window_.Tabs().GetActiveTab();
    if (!tab || tab->url.empty() || tab->url.rfind("fubuki://", 0) == 0 ||
        tab->url.rfind("data:", 0) == 0) {
      return BoolValue(false);
    }
    auto list = FrostInvoke("bookmarks.list", CefDictionaryValue::Create());
    if (list && list->GetType() == VTYPE_LIST) {
      auto records = list->GetList();
      for (size_t i = 0; i < records->GetSize(); ++i) {
        auto record = records->GetDictionary(i);
        if (record && record->GetString("url").ToString() == tab->url) {
          auto params = CefDictionaryValue::Create();
          params->SetString("url", tab->url);
          return FrostInvoke("bookmarks.remove", params);
        }
      }
    }
    auto params = CefDictionaryValue::Create();
    params->SetString("title", tab->title.empty() ? tab->url : tab->title);
    params->SetString("url", tab->url);
    params->SetString("faviconUrl", tab->faviconUrl);
    return FrostInvoke("bookmarks.save", params);
  };

  methods_["bookmarks.list"] = [this](CefRefPtr<CefDictionaryValue>) {
    return FrostInvoke("bookmarks.list", CefDictionaryValue::Create());
  };

  methods_["bookmarks.save"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return FrostInvoke("bookmarks.save", params);
  };

  methods_["bookmarks.remove"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return FrostInvoke("bookmarks.remove", params);
  };

  methods_["history.remove"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return FrostInvoke("history.remove", params);
  };

  methods_["history.list"] = [this](CefRefPtr<CefDictionaryValue>) {
    return FrostInvoke("history.list", CefDictionaryValue::Create());
  };

  methods_["history.clearRange"] =
      [this](CefRefPtr<CefDictionaryValue> params) {
        return FrostInvoke("history.clearRange", params);
      };
  methods_["history.clear"] = [this](CefRefPtr<CefDictionaryValue>) {
    auto params = CefDictionaryValue::Create();
    params->SetString("range", "all");
    return FrostInvoke("history.clearRange", params);
  };

  methods_["downloads.list"] = [this](CefRefPtr<CefDictionaryValue>) {
    return FrostInvoke("downloads.list", CefDictionaryValue::Create());
  };

  methods_["downloads.remove"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return FrostInvoke("downloads.remove", params);
  };
  methods_["downloads.clear"] = [this](CefRefPtr<CefDictionaryValue>) {
    auto params = CefDictionaryValue::Create();
    params->SetString("target", "downloads");
    return FrostInvoke("data.clear", params);
  };
  methods_["bookmarks.clear"] = [this](CefRefPtr<CefDictionaryValue>) {
    auto params = CefDictionaryValue::Create();
    params->SetString("target", "bookmarks");
    return FrostInvoke("data.clear", params);
  };

  methods_["downloads.open"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return BoolValue(window_.OpenDownloadedFile(params->GetString("path")));
  };

  methods_["downloads.reveal"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return BoolValue(window_.RevealDownloadedFile(params->GetString("path")));
  };

  methods_["data.clear"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return FrostInvoke("data.clear", params);
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
      return window_.SetLiveSidebarWidth(NumberValue(params, "width"));
    });
  };

  methods_["permissions.set"] = [this](CefRefPtr<CefDictionaryValue> params) {
    return FrostInvoke("permissions.set", params);
  };

  methods_["ui.setOverlayActive"] = [this](
                                        CefRefPtr<CefDictionaryValue> params) {
    const double overlayWidth = NumberValue(params, "width", 392.0);
    const double overlayHeight = NumberValue(params, "height", 560.0);
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
    // Protocol commands must enter through the same method table as toolbar
    // actions. Executing the native registry first used to create a host tab
    // and then create a second engine tab for the same user gesture.
    if (id != "commands.execute" && methods_.contains(id)) {
      return Invoke(id, args);
    }
    return window_.Commands().Execute(id, args);
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
    std::string message = "FrostEngine request failed";
    std::string code;
    if (response->HasKey("result") &&
        response->GetType("result") == VTYPE_STRING) {
      message = response->GetString("result").ToString();
    } else if (response->HasKey("result") &&
               response->GetType("result") == VTYPE_DICTIONARY) {
      auto error = response->GetDictionary("result");
      const std::string structuredMessage =
          error->GetString("message").ToString();
      message = structuredMessage.empty() ? message : structuredMessage;
      code = error->GetString("code").ToString();
    }
    auto value = ErrorValue(message);
    if (!code.empty()) {
      value->GetDictionary()->SetString("errorCode", code);
    }
    return value;
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
