#include "cef/FubukiClient.h"

#include <algorithm>
#include <atomic>
#include <cctype>
#include <sstream>
#include <vector>

#include "browser/BrowserAppController.h"
#include "browser/BrowserWindow.h"
#include "include/base/cef_callback.h"
#include "include/cef_parser.h"
#include "include/cef_task.h"
#include "include/wrapper/cef_closure_task.h"
#include "include/wrapper/cef_helpers.h"

namespace fubuki {

namespace {

std::string HtmlEscape(const std::string& value) {
  std::ostringstream out;
  for (const char c : value) {
    switch (c) {
      case '&':
        out << "&amp;";
        break;
      case '<':
        out << "&lt;";
        break;
      case '>':
        out << "&gt;";
        break;
      case '"':
        out << "&quot;";
        break;
      case '\'':
        out << "&#39;";
        break;
      default:
        out << c;
        break;
    }
  }
  return out.str();
}

bool StartsWith(const std::string& value, const std::string& prefix) {
  return value.rfind(prefix, 0) == 0;
}

bool IsTrustedSettingsActionSource(const std::string& url) {
  return url == "fubuki://settings" || StartsWith(url, "fubuki://settings/") ||
         url == "fubuki://bookmarks" || StartsWith(url, "fubuki://bookmarks/") ||
         url == "fubuki://history" || StartsWith(url, "fubuki://history/") ||
         url == "fubuki://downloads" || StartsWith(url, "fubuki://downloads/") ||
         url == "fubuki://debug" || StartsWith(url, "fubuki://debug/");
}

bool IsBlankPopupUrl(const std::string& url) {
  return url.empty() || url == "about:blank";
}

bool IsFubukiInternalUrl(const std::string& url) {
  return StartsWith(url, "fubuki://");
}

bool IsPermissionOrigin(const std::string& origin) {
  const bool http = StartsWith(origin, "http://");
  const bool https = StartsWith(origin, "https://");
  if ((!http && !https) || origin.size() <= (http ? 7U : 8U)) {
    return false;
  }
  const size_t authorityStart = http ? 7U : 8U;
  const size_t authorityEnd = origin.find_first_of("/?#", authorityStart);
  const std::string authority = origin.substr(authorityStart, authorityEnd == std::string::npos
                                                                  ? std::string::npos
                                                                  : authorityEnd - authorityStart);
  if (authority.empty() || authority.find('@') != std::string::npos) {
    return false;
  }
  return std::none_of(authority.begin(), authority.end(),
                      [](unsigned char c) { return std::isspace(c) || std::iscntrl(c); });
}

static_assert(CEF_PERMISSION_TYPE_CAMERA_PAN_TILT_ZOOM == (1u << 1));
static_assert(CEF_PERMISSION_TYPE_CAMERA_STREAM == (1u << 2));
static_assert(CEF_PERMISSION_TYPE_GEOLOCATION == (1u << 8));
static_assert(CEF_PERMISSION_TYPE_MIC_STREAM == (1u << 12));
static_assert(CEF_PERMISSION_TYPE_NOTIFICATIONS == (1u << 15));
static_assert(CEF_PERMISSION_TYPE_KEYBOARD_LOCK == (1u << 16));
static_assert(CEF_PERMISSION_TYPE_POINTER_LOCK == (1u << 17));
static_assert(CEF_MEDIA_PERMISSION_DEVICE_AUDIO_CAPTURE == 1);
static_assert(CEF_MEDIA_PERMISSION_DEVICE_VIDEO_CAPTURE == 2);

std::string PermissionRequestedJson(const std::string& promptId, const std::string& tabId,
                                    const std::string& windowId, const std::string& origin,
                                    const std::vector<std::string>& permissions, bool isPrivate) {
  auto root = CefDictionaryValue::Create();
  root->SetInt("version", 0);
  root->SetString("event", "permission.requested");
  auto payload = CefDictionaryValue::Create();
  payload->SetString("promptId", promptId);
  payload->SetString("tabId", tabId);
  payload->SetString("windowId", windowId);
  payload->SetString("origin", origin);
  payload->SetBool("isPrivate", isPrivate);
  auto permissionList = CefListValue::Create();
  for (size_t index = 0; index < permissions.size(); ++index) {
    permissionList->SetString(index, permissions[index]);
  }
  payload->SetList("permissions", permissionList);
  root->SetDictionary("payload", payload);
  auto value = CefValue::Create();
  value->SetDictionary(root);
  return CefWriteJSON(value, JSON_WRITER_DEFAULT).ToString();
}

std::string PermissionDismissedJson(const std::string& promptId, const std::string& tabId,
                                    const std::string& windowId) {
  auto root = CefDictionaryValue::Create();
  root->SetInt("version", 0);
  root->SetString("event", "permission.dismissed");
  auto payload = CefDictionaryValue::Create();
  payload->SetString("promptId", promptId);
  payload->SetString("tabId", tabId);
  payload->SetString("windowId", windowId);
  root->SetDictionary("payload", payload);
  auto value = CefValue::Create();
  value->SetDictionary(root);
  return CefWriteJSON(value, JSON_WRITER_DEFAULT).ToString();
}

std::atomic<uint64_t> gNextMediaPromptId{1};

std::string DecodeFormValue(const std::string& value) {
  return CefURIDecode(value, true,
                      static_cast<cef_uri_unescape_rule_t>(
                          UU_SPACES | UU_PATH_SEPARATORS |
                          UU_URL_SPECIAL_CHARS_EXCEPT_PATH_SEPARATORS | UU_REPLACE_PLUS_WITH_SPACE))
      .ToString();
}

std::string FormParam(const std::string& encoded, const std::string& key) {
  size_t start = 0;
  while (start <= encoded.size()) {
    const size_t end = encoded.find('&', start);
    const std::string pair =
        encoded.substr(start, end == std::string::npos ? std::string::npos : end - start);
    const size_t equals = pair.find('=');
    const std::string name = equals == std::string::npos ? pair : pair.substr(0, equals);
    if (DecodeFormValue(name) == key) {
      const std::string value = equals == std::string::npos ? "" : pair.substr(equals + 1);
      return DecodeFormValue(value);
    }
    if (end == std::string::npos) {
      break;
    }
    start = end + 1;
  }
  return "";
}

std::string QueryString(const std::string& url) {
  const size_t queryStart = url.find('?');
  return queryStart == std::string::npos ? "" : url.substr(queryStart + 1);
}

bool IsDestructiveSettingsAction(const std::string& key) {
  return key == "removeBookmark" || key == "removeHistory" || key == "removeDownload" ||
         key == "openDownload" || key == "revealDownload" || key == "openDevTools" ||
         key == "clearData" || key == "clearHistoryRange" || key == "resetSetting" ||
         key == "setPermission" || key == "removePermission";
}

std::string PostBody(CefRefPtr<CefRequest> request) {
  if (!request || !request->GetPostData()) {
    return "";
  }
  std::vector<CefRefPtr<CefPostDataElement>> elements;
  request->GetPostData()->GetElements(elements);
  std::string body;
  for (auto element : elements) {
    if (!element || element->GetType() != PDE_TYPE_BYTES) {
      continue;
    }
    const size_t size = element->GetBytesCount();
    if (size == 0) {
      continue;
    }
    const size_t offset = body.size();
    body.resize(offset + size);
    element->GetBytes(size, body.data() + offset);
  }
  return body;
}

std::string BrowserAppearance(BrowserWindow* window) {
  if (!window) {
    return "system";
  }
  const std::string appearance = window->Store().GetSetting("appearance");
  if (appearance == "light" || appearance == "dark") {
    return appearance;
  }
  return "system";
}

}  // namespace

BrowserWindow* FubukiClient::Window() const {
  auto* app = GetBrowserAppController();
  return app ? app->FindWindow(windowId_) : nullptr;
}

FubukiClient::FubukiClient(BrowserWindow* window, std::string tabId, bool isUi)
    : windowId_(window->WindowId()), tabId_(std::move(tabId)), isUi_(isUi) {
  if (isUi_) {
    CefMessageRouterConfig config;
    config.js_query_function = "cefQuery";
    config.js_cancel_function = "cefQueryCancel";
    messageRouter_ = CefMessageRouterBrowserSide::Create(config);
    messageRouter_->AddHandler(window->Bridge(), false);
  }
}

bool FubukiClient::OnProcessMessageReceived(CefRefPtr<CefBrowser> browser,
                                            CefRefPtr<CefFrame> frame, CefProcessId source_process,
                                            CefRefPtr<CefProcessMessage> message) {
  CEF_REQUIRE_UI_THREAD();
  if (Window() && messageRouter_ &&
      messageRouter_->OnProcessMessageReceived(browser, frame, source_process, message)) {
    return true;
  }
  return false;
}

void FubukiClient::OnAfterCreated(CefRefPtr<CefBrowser> browser) {
  CEF_REQUIRE_UI_THREAD();
  if (!Window()) {
    return;
  }
  if (isUi_) {
    Window()->SetUiBrowser(browser);
  } else {
    Window()->OnTabBrowserCreated(tabId_, browser);
  }
}

bool FubukiClient::OnBeforePopup(CefRefPtr<CefBrowser>, CefRefPtr<CefFrame> frame, int,
                                 const CefString& target_url, const CefString&,
                                 WindowOpenDisposition, bool user_gesture, const CefPopupFeatures&,
                                 CefWindowInfo& windowInfo, CefRefPtr<CefClient>& client,
                                 CefBrowserSettings& settings, CefRefPtr<CefDictionaryValue>&,
                                 bool* no_javascript_access) {
  CEF_REQUIRE_UI_THREAD();
  if (!Window() || isUi_) {
    return true;
  }
  const std::string url = target_url.ToString();
  const std::string sourceUrl = frame ? frame->GetURL().ToString() : "";
  if (!user_gesture || IsFubukiInternalUrl(sourceUrl)) {
    if (!Window()->IsPrivate()) {
      Window()->Store().AddLog(
          "info", "Blocked popup from " + (sourceUrl.empty() ? "unknown source" : sourceUrl));
    }
    return true;
  }
  if (IsBlankPopupUrl(url)) {
    const std::string popupTabId = Window()->CreatePendingPopupTab("about:blank", true);
    if (popupTabId.empty()) {
      return true;
    }
    if (!Window()->IsPrivate()) {
      Window()->Store().AddLog("info", "Opened blank popup as pending tab: " + popupTabId);
    }
    windowInfo = Window()->PopupWindowInfo();
    client = new FubukiClient(Window(), popupTabId, false);
    settings.background_color = CefColorSetARGB(255, 255, 255, 255);
    if (no_javascript_access) {
      *no_javascript_access = false;
    }
    const std::string windowId = Window()->WindowId();
    CefPostDelayedTask(TID_UI,
                       base::BindOnce(
                           [](std::string windowId, std::string tabId) {
                             BrowserAppController* app = GetBrowserAppController();
                             if (!app) {
                               return;
                             }
                             for (auto* window : app->Windows()) {
                               if (window && window->WindowId() == windowId) {
                                 window->ExpirePendingPopupTab(tabId);
                                 return;
                               }
                             }
                           },
                           windowId, popupTabId),
                       15000);
    return false;
  }
  if (!Window()->IsPrivate()) {
    Window()->Store().AddLog("info", "Opened popup in new tab: " + url);
  }
  const std::string windowId = Window()->WindowId();
  CefPostTask(TID_UI, base::BindOnce(
                          [](std::string windowId, std::string url) {
                            BrowserAppController* app = GetBrowserAppController();
                            if (!app) {
                              return;
                            }
                            for (auto* window : app->Windows()) {
                              if (window && window->WindowId() == windowId) {
                                window->CreateTab(url, true);
                                return;
                              }
                            }
                          },
                          windowId, url));
  return true;
}

bool FubukiClient::DoClose(CefRefPtr<CefBrowser>) {
  return false;
}

void FubukiClient::OnBeforeClose(CefRefPtr<CefBrowser> browser) {
  CEF_REQUIRE_UI_THREAD();
  CancelPendingPermissions();
  if (messageRouter_)
    messageRouter_->OnBeforeClose(browser);
}

void FubukiClient::OnLoadingStateChange(CefRefPtr<CefBrowser>, bool isLoading, bool canGoBack,
                                        bool canGoForward) {
  if (!isUi_ && Window()) {
    Window()->OnTabLoadingState(tabId_, isLoading, canGoBack, canGoForward);
  }
}

void FubukiClient::OnLoadStart(CefRefPtr<CefBrowser>, CefRefPtr<CefFrame> frame, TransitionType) {
  if (!isUi_ && Window() && frame->IsMain()) {
    Window()->OnNavigationStarted(tabId_);
  }
}

void FubukiClient::OnLoadEnd(CefRefPtr<CefBrowser>, CefRefPtr<CefFrame> frame, int) {
  if (!isUi_ && Window() && frame->IsMain()) {
    Window()->OnNavigationFinished(tabId_);
  }
}

void FubukiClient::OnLoadError(CefRefPtr<CefBrowser>, CefRefPtr<CefFrame> frame,
                               ErrorCode errorCode, const CefString& errorText,
                               const CefString& failedUrl) {
  if (!isUi_ && Window() && frame->IsMain() && errorCode != ERR_ABORTED) {
    const std::string message = errorText.ToString();
    const std::string failed = failedUrl.ToString();
    const std::string appearance = BrowserAppearance(Window());
    Window()->OnNavigationFailed(tabId_, message);
    const std::string html =
        "<!doctype html><html data-appearance=\"" + HtmlEscape(appearance) +
        "\"><meta charset=\"utf-8\"><title>Page load failed</title>"
        "<style>*{box-sizing:border-box}@keyframes "
        "pageIn{from{opacity:0;transform:translateY(10px)}to{opacity:1;"
        "transform:translateY(0)}}"
        "body{font:15px -apple-system,BlinkMacSystemFont,'SF Pro "
        "Text','Helvetica "
        "Neue',sans-serif;margin:0;padding:48px;background:#f5f6f8;color:#"
        "15171a}"
        "html[data-appearance=dark] "
        "body{background:#14161a;color:#f4f6f8;color-scheme:dark}"
        "main{max-width:760px;animation:pageIn .32s "
        "cubic-bezier(.2,.8,.2,1)}h1{font-size:28px;line-height:1.1;margin:0 0 "
        "12px;font-weight:720}p{line-height:1.5;color:#66707c}"
        "html[data-appearance=dark] p{color:#a7b0bd}"
        "code{display:inline-block;max-width:100%;background:#fff;border:1px "
        "solid rgb(22 28 36/.12);padding:3px "
        "6px;border-radius:6px;word-break:break-all}"
        ".actions{display:flex;flex-wrap:wrap;gap:8px;margin-top:20px}button,a{"
        "border:1px solid rgb(22 28 "
        "36/.14);border-radius:7px;background:#fff;color:#15171a;"
        "font:inherit;font-weight:620;padding:7px "
        "12px;text-decoration:none;transition:background .16s ease,transform "
        ".16s ease}button:hover,a:hover{background:rgb(22 28 "
        "36/.055);transform:translateY(-1px)}"
        "html[data-appearance=dark] code,html[data-appearance=dark] "
        "button,html[data-appearance=dark] "
        "a{background:#1d2025;color:#f4f6f8;border-color:rgb(255 255 255/.12)}"
        "@media(prefers-color-scheme:dark){html[data-appearance=system] "
        "body{background:#14161a;color:#f4f6f8;color-scheme:dark}html[data-"
        "appearance=system] p{color:#a7b0bd}html[data-appearance=system] "
        "code,html[data-appearance=system] button,html[data-appearance=system] "
        "a{background:#1d2025;color:#f4f6f8;border-color:rgb(255 255 255/.12)}}"
        "@media(prefers-reduced-motion:reduce){*,*::before,*::after{animation:"
        "none!important;transition:none!important}}"
        "html[data-appearance=dark] "
        "body{background:#14161a;color:#f4f6f8;color-scheme:dark}"
        "html[data-appearance=dark] p{color:#a7b0bd}"
        "html[data-appearance=dark] code,html[data-appearance=dark] "
        "button,html[data-appearance=dark] "
        "a{background:#1d2025;color:#f4f6f8;border-color:rgb(255 255 255/.12)}"
        "</style>"
        "<main><h1>Page load failed</h1><p>" +
        HtmlEscape(message) + "</p><p>Check the URL, reload the page, or go back.</p><p><code>" +
        HtmlEscape(failed) + "</code></p><div class=\"actions\"><a href=\"" + HtmlEscape(failed) +
        "\">Reload</a><button onclick=\"history.back()\">Back</button><a "
        "href=\"fubuki://newtab/\">New tab</a></div></main></html>";
    frame->LoadURL("data:text/html;charset=utf-8," + CefURIEncode(html, false).ToString());
  }
}

void FubukiClient::OnTitleChange(CefRefPtr<CefBrowser>, const CefString& title) {
  if (!isUi_ && Window()) {
    Window()->OnTabTitle(tabId_, title.ToString());
  }
}

void FubukiClient::OnAddressChange(CefRefPtr<CefBrowser>, CefRefPtr<CefFrame> frame,
                                   const CefString& url) {
  if (!isUi_ && Window() && frame->IsMain()) {
    Window()->OnTabUrl(tabId_, url.ToString());
  }
}

void FubukiClient::OnFaviconURLChange(CefRefPtr<CefBrowser>,
                                      const std::vector<CefString>& icon_urls) {
  if (!isUi_ && Window() && !icon_urls.empty()) {
    Window()->OnTabFavicon(tabId_, icon_urls.front().ToString());
  }
}

bool FubukiClient::OnBeforeDownload(CefRefPtr<CefBrowser>, CefRefPtr<CefDownloadItem> download_item,
                                    const CefString& suggested_name,
                                    CefRefPtr<CefBeforeDownloadCallback> callback) {
  if (!Window() || !download_item || !callback) {
    return false;
  }
  const std::string path = Window()->DownloadPathFor(suggested_name.ToString());
  const bool askBeforeDownload = Window()->Store().GetSetting("askBeforeDownload") == "on";
  Window()->OnDownloadStarted(std::to_string(download_item->GetId()),
                              download_item->GetURL().ToString(), path);
  callback->Continue(path, askBeforeDownload);
  return true;
}

void FubukiClient::OnDownloadUpdated(CefRefPtr<CefBrowser>,
                                     CefRefPtr<CefDownloadItem> download_item,
                                     CefRefPtr<CefDownloadItemCallback>) {
  if (!Window() || !download_item) {
    return;
  }
  const int percent = download_item->GetPercentComplete();
  std::string state = "in_progress";
  if (download_item->IsComplete()) {
    state = "completed";
  } else if (download_item->IsCanceled()) {
    state = "canceled";
  } else if (download_item->IsInterrupted()) {
    state = "failed";
  } else if (percent >= 100) {
    state = "completed";
  }
  Window()->OnDownloadUpdated(std::to_string(download_item->GetId()),
                              download_item->GetURL().ToString(),
                              download_item->GetFullPath().ToString(), state, percent);
}

bool FubukiClient::OnPreKeyEvent(CefRefPtr<CefBrowser>, const CefKeyEvent& event, CefEventHandle,
                                 bool* is_keyboard_shortcut) {
  if (!Window() || event.type != KEYEVENT_RAWKEYDOWN) {
    return false;
  }
  const bool commandDown = (event.modifiers & EVENTFLAG_COMMAND_DOWN) != 0;
  const bool controlDown = (event.modifiers & EVENTFLAG_CONTROL_DOWN) != 0;
  const bool altDown = (event.modifiers & EVENTFLAG_ALT_DOWN) != 0;
  const bool shiftDown = (event.modifiers & EVENTFLAG_SHIFT_DOWN) != 0;
  const char character = static_cast<char>(event.unmodified_character);
  const bool handled = Window()->HandleShortcut(commandDown, controlDown, altDown, shiftDown,
                                                event.windows_key_code, character, tabId_);
  if (handled && is_keyboard_shortcut) {
    *is_keyboard_shortcut = true;
  }
  return handled;
}

bool FubukiClient::OnBeforeBrowse(CefRefPtr<CefBrowser>, CefRefPtr<CefFrame> frame,
                                  CefRefPtr<CefRequest> request, bool user_gesture,
                                  bool is_redirect) {
  if (!Window() || isUi_ || !frame || !frame->IsMain() || !request) {
    return false;
  }
  CancelPendingPermissions();
  const std::string url = request->GetURL().ToString();
  if (StartsWith(url, "fubuki://settings/set")) {
    const std::string method = request->GetMethod().ToString();
    // CEF does not consistently mark HTML form POST navigations as a user
    // gesture and can omit POST elements for custom-scheme navigation. Trust
    // only internal-page sources, and use the duplicated URL query when the
    // POST body is unavailable.
    const std::string frameUrl = frame->GetURL().ToString();
    const std::string referrerUrl = request->GetReferrerURL().ToString();
    const bool trustedSource =
        IsTrustedSettingsActionSource(frameUrl) || IsTrustedSettingsActionSource(referrerUrl);
    if (is_redirect || !trustedSource || (method != "POST" && !user_gesture)) {
      if (!Window()->IsPrivate()) {
        Window()->Store().AddLog("warning", "Blocked settings action: method=" + method +
                                                " source=" + frameUrl + " referrer=" + referrerUrl);
      }
      return true;
    }
    const std::string postBody = method == "POST" ? PostBody(request) : "";
    const std::string query = postBody.empty() ? QueryString(url) : postBody;
    const std::string key = FormParam(query, "key");
    if (key.empty()) {
      if (!Window()->IsPrivate()) {
        Window()->Store().AddLog("warning", "Blocked settings action with empty form body");
      }
      return true;
    }
    if (method != "POST" && IsDestructiveSettingsAction(key)) {
      if (!Window()->IsPrivate()) {
        Window()->Store().AddLog("warning", "Blocked destructive settings action over GET: " + key);
      }
      return true;
    }
    Window()->HandleSettingsUrl(tabId_, "fubuki://settings/set?" + query);
    return true;
  }
  if (StartsWith(url, "fubuki://newtab/search")) {
    Window()->HandleNewTabSearchUrl(tabId_, url);
    return true;
  }
  return false;
}

void FubukiClient::OnDraggableRegionsChanged(CefRefPtr<CefBrowser>, CefRefPtr<CefFrame>,
                                             const std::vector<CefDraggableRegion>& regions) {
  if (isUi_ && Window()) {
    Window()->OnUiDraggableRegionsChanged(regions);
  }
}

bool FubukiClient::OnShowPermissionPrompt(CefRefPtr<CefBrowser> browser, uint64_t prompt_id,
                                          const CefString& requesting_origin,
                                          uint32_t requested_permissions,
                                          CefRefPtr<CefPermissionPromptCallback> callback) {
  CEF_REQUIRE_UI_THREAD();
  const std::string origin = requesting_origin.ToString();
  const std::string id = std::to_string(browser->GetIdentifier()) + ":" + std::to_string(prompt_id);
  std::vector<std::string> permissions;
  if (!callback || !Window() || isUi_ || !IsPermissionOrigin(origin) ||
      IsFubukiInternalUrl(origin) || !PermissionNames(requested_permissions, permissions) ||
      pendingPermissions_.Contains(id)) {
    if (callback) {
      callback->Continue(CEF_PERMISSION_RESULT_DENY);
    }
    return true;
  }

  const auto token = pendingPermissions_.Add(id, [this, id, callback](const std::string& decision) {
    callback->Continue(decision == "allow" ? CEF_PERMISSION_RESULT_ACCEPT
                       : decision == "ask" ? CEF_PERMISSION_RESULT_DISMISS
                                           : CEF_PERMISSION_RESULT_DENY);
    if (auto* window = Window())
      window->PushHostEventJson(PermissionDismissedJson(id, tabId_, windowId_));
  });
  if (!Window()->PushHostEventJson(PermissionRequestedJson(id, tabId_, Window()->WindowId(), origin,
                                                           permissions, Window()->IsPrivate()))) {
    pendingPermissions_.Resolve(id, "block");
    return true;
  }
  SchedulePermissionTimeout(id, token);
  return true;
}

bool FubukiClient::OnRequestMediaAccessPermission(CefRefPtr<CefBrowser>, CefRefPtr<CefFrame>,
                                                  const CefString& requesting_origin,
                                                  uint32_t requested_permissions,
                                                  CefRefPtr<CefMediaAccessCallback> callback) {
  CEF_REQUIRE_UI_THREAD();
  const std::string origin = requesting_origin.ToString();
  std::vector<std::string> permissions;
  if (!callback || !Window() || isUi_ || !IsPermissionOrigin(origin) ||
      IsFubukiInternalUrl(origin) || !MediaPermissionNames(requested_permissions, permissions)) {
    if (callback) {
      callback->Continue(CEF_MEDIA_PERMISSION_NONE);
    }
    return true;
  }

  const std::string id = "media-" + std::to_string(gNextMediaPromptId.fetch_add(1));
  const auto token = pendingPermissions_.Add(id, [this, id, callback, requested_permissions](
                                                     const std::string& decision) {
    if (decision == "ask")
      callback->Cancel();
    else
      callback->Continue(decision == "allow" ? requested_permissions : CEF_MEDIA_PERMISSION_NONE);
    if (auto* window = Window())
      window->PushHostEventJson(PermissionDismissedJson(id, tabId_, windowId_));
  });
  if (!Window()->PushHostEventJson(PermissionRequestedJson(id, tabId_, Window()->WindowId(), origin,
                                                           permissions, Window()->IsPrivate()))) {
    pendingPermissions_.Resolve(id, "block");
    return true;
  }
  SchedulePermissionTimeout(id, token);
  return true;
}

void FubukiClient::OnDismissPermissionPrompt(CefRefPtr<CefBrowser> browser, uint64_t prompt_id,
                                             cef_permission_request_result_t) {
  CEF_REQUIRE_UI_THREAD();
  const std::string id = std::to_string(browser->GetIdentifier()) + ":" + std::to_string(prompt_id);
  if (pendingPermissions_.Remove(id) && Window()) {
    Window()->PushHostEventJson(PermissionDismissedJson(id, tabId_, Window()->WindowId()));
  }
}

bool FubukiClient::ResolvePermission(const std::string& promptId, const std::string& decision,
                                     uint64_t token) {
  CEF_REQUIRE_UI_THREAD();
  return pendingPermissions_.Resolve(promptId, decision, token);
}

void FubukiClient::CancelPendingPermissions() {
  pendingPermissions_.Cancel();
}

void FubukiClient::SchedulePermissionTimeout(const std::string& promptId, uint64_t token) {
  CefRefPtr<FubukiClient> self(this);
  CefPostDelayedTask(
      TID_UI,
      base::BindOnce([](CefRefPtr<FubukiClient> client, std::string id,
                        uint64_t token) { client->ResolvePermission(id, "block", token); },
                     self, promptId, token),
      60000);
}

}  // namespace fubuki
