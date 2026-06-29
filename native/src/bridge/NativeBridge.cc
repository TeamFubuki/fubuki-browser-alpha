#include "bridge/NativeBridge.h"

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

CefRefPtr<CefListValue> CopyListOrEmpty(CefRefPtr<CefListValue> value) {
  return value ? value->Copy() : CefListValue::Create();
}

CefRefPtr<CefDictionaryValue> CopyDictionaryOrEmpty(CefRefPtr<CefDictionaryValue> value) {
  return value ? value->Copy(false) : CefDictionaryValue::Create();
}

}  // namespace

NativeBridge::NativeBridge(BrowserWindow& window) : window_(window) {}

bool NativeBridge::OnQuery(CefRefPtr<CefBrowser>,
                           CefRefPtr<CefFrame> frame,
                           int64_t,
                           const CefString& request,
                           bool,
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

void NativeBridge::OnQueryCanceled(CefRefPtr<CefBrowser>, CefRefPtr<CefFrame>, int64_t) {}

CefRefPtr<CefValue> NativeBridge::Invoke(const std::string& method, CefRefPtr<CefDictionaryValue> params) {
  if (method == "app.getState") {
    return StateValue();
  }
  if (method == "tabs.create") {
    const std::string url = params->HasKey("url") ? params->GetString("url") : "fubuki://newtab/";
    const bool active = !params->HasKey("active") || params->GetBool("active");
    return BoolValue(window_.CreateTab(url, active));
  }
  if (method == "tabs.activate") {
    return BoolValue(window_.ActivateTab(params->GetString("tabId")));
  }
  if (method == "tabs.close") {
    return BoolValue(window_.CloseTab(params->GetString("tabId")));
  }
  if (method == "tabs.navigate") {
    return BoolValue(window_.Navigate(params->GetString("tabId"), params->GetString("input")));
  }
  if (method == "tabs.reload") {
    return BoolValue(window_.Reload(params->GetString("tabId")));
  }
  if (method == "tabs.stop") {
    return BoolValue(window_.Stop(params->GetString("tabId")));
  }
  if (method == "tabs.goBack") {
    return BoolValue(window_.GoBack(params->GetString("tabId")));
  }
  if (method == "tabs.goForward") {
    return BoolValue(window_.GoForward(params->GetString("tabId")));
  }
  if (method == "bookmarks.addActive") {
    return BoolValue(window_.AddActiveBookmark());
  }
  if (method == "bookmarks.remove") {
    return BoolValue(window_.RemoveBookmark(params->GetString("url")));
  }
  if (method == "settings.set") {
    return BoolValue(window_.SetSetting(params->GetString("key"), params->GetString("value")));
  }
  if (method == "ui.setOverlayActive") {
    return BoolValue(window_.SetUiOverlayActive(params->HasKey("active") && params->GetBool("active")));
  }
  if (method == "commands.execute") {
    const std::string id = params->GetString("id");
    CefRefPtr<CefDictionaryValue> args = CefDictionaryValue::Create();
    if (params->HasKey("args") && params->GetType("args") == VTYPE_DICTIONARY) {
      args = params->GetDictionary("args");
    }
    return window_.Commands().Execute(id, args);
  }
  return ErrorValue("Unknown bridge method: " + method);
}

void NativeBridge::EmitToUi(const std::string& eventName, CefRefPtr<CefDictionaryValue> payload) {
  CEF_REQUIRE_UI_THREAD();
  auto event = CefDictionaryValue::Create();
  event->SetString("name", eventName);
  event->SetDictionary("payload", payload);
  auto value = CefValue::Create();
  value->SetDictionary(event);

  const std::string script = "window.dispatchEvent(new CustomEvent('fubuki:event', { detail: " + WriteValue(value) + " }));";
  if (auto ui = window_.UiBrowser()) {
    ui->GetMainFrame()->ExecuteJavaScript(script, "fubuki://app/", 0);
  }
}

std::string NativeBridge::GetStateJson() const {
  return WriteValue(StateValue());
}

CefRefPtr<CefValue> NativeBridge::ErrorValue(const std::string& message) const {
  auto dict = CefDictionaryValue::Create();
  dict->SetBool("ok", false);
  dict->SetString("error", message);
  auto value = CefValue::Create();
  value->SetDictionary(dict);
  return value;
}

CefRefPtr<CefDictionaryValue> NativeBridge::TabToDictionary(const Tab& tab) const {
  auto dict = CefDictionaryValue::Create();
  dict->SetString("id", tab.id);
  dict->SetString("title", tab.title);
  dict->SetString("url", tab.url);
  dict->SetString("faviconUrl", tab.faviconUrl);
  dict->SetString("errorText", tab.errorText);
  dict->SetBool("isLoading", tab.isLoading);
  dict->SetBool("canGoBack", tab.canGoBack);
  dict->SetBool("canGoForward", tab.canGoForward);
  dict->SetBool("isActive", tab.isActive);
  return dict;
}

CefRefPtr<CefValue> NativeBridge::StateValue() const {
  auto state = CefDictionaryValue::Create();
  auto tabs = CefListValue::Create();
  const auto snapshot = window_.Tabs().GetTabs();
  for (size_t i = 0; i < snapshot.size(); ++i) {
    tabs->SetDictionary(i, TabToDictionary(snapshot[i]));
  }
  state->SetString("bridgeVersion", "1");
  state->SetString("activeTabId", window_.Tabs().GetActiveTabId());
  state->SetString("profilePath", window_.Store().ProfilePath());
  state->SetList("tabs", tabs);
  state->SetList("history", CopyListOrEmpty(window_.Store().History()));
  state->SetList("bookmarks", CopyListOrEmpty(window_.Store().Bookmarks()));
  state->SetList("downloads", CopyListOrEmpty(window_.Store().Downloads()));
  state->SetList("logs", CopyListOrEmpty(window_.Store().Logs()));
  state->SetDictionary("settings", CopyDictionaryOrEmpty(window_.Store().Settings()));
  auto value = CefValue::Create();
  value->SetDictionary(state);
  return value;
}

std::string NativeBridge::WriteValue(CefRefPtr<CefValue> value) const {
  return CefWriteJSON(value, JSON_WRITER_DEFAULT).ToString();
}

}  // namespace fubuki
