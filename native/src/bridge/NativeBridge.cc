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
  if (method == "tabs.pin") {
    return BoolValue(window_.PinTab(params->GetString("tabId"), params->HasKey("pinned") && params->GetBool("pinned")));
  }
  if (method == "tabs.duplicate") {
    return BoolValue(window_.DuplicateTab(params->GetString("tabId")));
  }
  if (method == "tabs.reopenClosed") {
    return BoolValue(window_.ReopenClosedTab());
  }
  if (method == "tabs.closeOther") {
    return BoolValue(window_.CloseOtherTabs(params->GetString("tabId")));
  }
  if (method == "tabs.closeToRight") {
    return BoolValue(window_.CloseTabsToRight(params->GetString("tabId")));
  }
  if (method == "tabs.move") {
    return BoolValue(window_.MoveTab(params->GetString("tabId"), params->GetInt("toIndex")));
  }
  if (method == "tabs.moveToNewWindow") {
    return BoolValue(window_.MoveTabToNewWindow(params->GetString("tabId")));
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
  if (method == "tabs.home") {
    return BoolValue(window_.GoHome());
  }
  if (method == "windows.create") {
    return BoolValue(window_.App().RequestNewWindow(false, nullptr));
  }
  if (method == "windows.createPrivate") {
    return BoolValue(window_.App().RequestNewPrivateWindow());
  }
  if (method == "windows.close") {
    return BoolValue(window_.CloseWindow());
  }
  if (method == "windows.reopenClosed") {
    return BoolValue(window_.App().ReopenClosedWindow());
  }
  if (method == "page.find") {
    return BoolValue(window_.FindInPage(params->GetString("query"), !params->HasKey("forward") || params->GetBool("forward")));
  }
  if (method == "page.stopFinding") {
    return BoolValue(window_.StopFinding(!params->HasKey("clear") || params->GetBool("clear")));
  }
  if (method == "page.zoomIn") {
    return BoolValue(window_.ZoomIn());
  }
  if (method == "page.zoomOut") {
    return BoolValue(window_.ZoomOut());
  }
  if (method == "page.zoomReset") {
    return BoolValue(window_.ResetZoom());
  }
  if (method == "page.print") {
    return BoolValue(window_.PrintPage());
  }
  if (method == "page.viewSource") {
    return BoolValue(window_.ViewSource());
  }
  if (method == "bookmarks.addActive") {
    return BoolValue(window_.AddActiveBookmark());
  }
  if (method == "bookmarks.save") {
    return BoolValue(window_.SaveBookmark(params->GetString("title"), params->GetString("url"), params->GetString("faviconUrl")));
  }
  if (method == "bookmarks.remove") {
    return BoolValue(window_.RemoveBookmark(params->GetString("url")));
  }
  if (method == "history.remove") {
    return BoolValue(window_.RemoveHistory(params->GetString("url")));
  }
  if (method == "history.clearRange") {
    return BoolValue(window_.ClearHistoryRange(params->GetString("range")));
  }
  if (method == "downloads.remove") {
    return BoolValue(window_.RemoveDownload(params->GetString("url"), params->GetString("path")));
  }
  if (method == "downloads.open") {
    return BoolValue(window_.OpenDownloadedFile(params->GetString("path")));
  }
  if (method == "downloads.reveal") {
    return BoolValue(window_.RevealDownloadedFile(params->GetString("path")));
  }
  if (method == "data.clear") {
    return BoolValue(window_.ClearBrowsingData(params->GetString("target")));
  }
  if (method == "settings.set") {
    return BoolValue(window_.SetSetting(params->GetString("key"), params->GetString("value")));
  }
  if (method == "settings.reset") {
    return BoolValue(window_.ResetSetting(params->GetString("key")));
  }
  if (method == "ui.setSidebarWidth") {
    return BoolValue(window_.SetLiveSidebarWidth(params->GetDouble("width")));
  }
  if (method == "permissions.set") {
    return BoolValue(window_.SetPermission(params->GetString("origin"), params->GetString("permission"), params->GetString("value")));
  }
  if (method == "ui.setOverlayActive") {
    const double overlayWidth = params->HasKey("width") ? params->GetDouble("width") : 392.0;
    const double overlayHeight = params->HasKey("height") ? params->GetDouble("height") : 560.0;
    return BoolValue(window_.SetUiOverlayActive(params->HasKey("active") && params->GetBool("active"), overlayWidth, overlayHeight));
  }
  if (method == "app.openDevTools") {
    return BoolValue(window_.OpenDevTools());
  }
  if (method == "commands.execute") {
    const std::string id = params->GetString("id");
    CefRefPtr<CefDictionaryValue> args = CefDictionaryValue::Create();
    if (params->HasKey("args") && params->GetType("args") == VTYPE_DICTIONARY) {
      args = params->GetDictionary("args");
    }
    return window_.Commands().Execute(id, args);
  }
  if (method == "commands.list") {
    auto value = CefValue::Create();
    value->SetList(window_.Commands().List());
    return value;
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
  dict->SetDouble("zoomLevel", tab.zoomLevel);
  dict->SetBool("isLoading", tab.isLoading);
  dict->SetBool("canGoBack", tab.canGoBack);
  dict->SetBool("canGoForward", tab.canGoForward);
  dict->SetBool("isActive", tab.isActive);
  dict->SetBool("isPinned", tab.isPinned);
  return dict;
}

CefRefPtr<CefValue> NativeBridge::StateValue() const {
  auto state = CefDictionaryValue::Create();
  auto tabs = CefListValue::Create();
  const auto snapshot = window_.Tabs().GetTabs();
  for (size_t i = 0; i < snapshot.size(); ++i) {
    tabs->SetDictionary(i, TabToDictionary(snapshot[i]));
  }
  auto windows = CefListValue::Create();
  const auto windowSnapshot = window_.App().Windows();
  for (size_t i = 0; i < windowSnapshot.size(); ++i) {
    windows->SetDictionary(i, windowSnapshot[i]->SessionSnapshot());
  }
  auto events = CefListValue::Create();
  const auto recentEvents = window_.App().Events().RecentEvents();
  for (size_t i = 0; i < recentEvents.size(); ++i) {
    auto item = CefDictionaryValue::Create();
    item->SetString("name", recentEvents[i].name);
    item->SetString("windowId", recentEvents[i].windowId);
    item->SetString("tabId", recentEvents[i].tabId);
    item->SetString("message", recentEvents[i].message);
    events->SetDictionary(i, item);
  }
  state->SetString("bridgeVersion", "1");
  state->SetString("windowId", window_.WindowId());
  state->SetBool("isPrivate", window_.IsPrivate());
  state->SetString("activeTabId", window_.Tabs().GetActiveTabId());
  state->SetString("profilePath", window_.Store().ProfilePath());
  state->SetList("tabs", tabs);
  state->SetList("windows", windows);
  state->SetList("history", CopyListOrEmpty(window_.Store().History()));
  state->SetList("bookmarks", CopyListOrEmpty(window_.Store().Bookmarks()));
  state->SetList("downloads", CopyListOrEmpty(window_.Store().Downloads()));
  state->SetList("permissions", CopyListOrEmpty(window_.Store().Permissions()));
  state->SetList("logs", CopyListOrEmpty(window_.Store().Logs()));
  state->SetList("commands", window_.Commands().List());
  state->SetList("recentEvents", events);
  state->SetDictionary("settings", CopyDictionaryOrEmpty(window_.Store().Settings()));
  auto value = CefValue::Create();
  value->SetDictionary(state);
  return value;
}

std::string NativeBridge::WriteValue(CefRefPtr<CefValue> value) const {
  return CefWriteJSON(value, JSON_WRITER_DEFAULT).ToString();
}

}  // namespace fubuki
