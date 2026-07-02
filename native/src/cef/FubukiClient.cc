#include "cef/FubukiClient.h"

#include "browser/BrowserWindow.h"
#include "include/cef_parser.h"
#include "include/wrapper/cef_helpers.h"

#include <sstream>

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

}  // namespace

FubukiClient::FubukiClient(BrowserWindow* window, std::string tabId, bool isUi)
    : window_(window), tabId_(std::move(tabId)), isUi_(isUi) {
  if (isUi_) {
    CefMessageRouterConfig config;
    config.js_query_function = "cefQuery";
    config.js_cancel_function = "cefQueryCancel";
    messageRouter_ = CefMessageRouterBrowserSide::Create(config);
    messageRouter_->AddHandler(window_->Bridge(), false);
  }
}

bool FubukiClient::OnProcessMessageReceived(CefRefPtr<CefBrowser> browser,
                                            CefRefPtr<CefFrame> frame,
                                            CefProcessId source_process,
                                            CefRefPtr<CefProcessMessage> message) {
  CEF_REQUIRE_UI_THREAD();
  if (messageRouter_ && messageRouter_->OnProcessMessageReceived(browser, frame, source_process, message)) {
    return true;
  }
  return false;
}

void FubukiClient::OnAfterCreated(CefRefPtr<CefBrowser> browser) {
  CEF_REQUIRE_UI_THREAD();
  if (!window_) {
    return;
  }
  if (isUi_) {
    window_->SetUiBrowser(browser);
  } else {
    window_->OnTabBrowserCreated(tabId_, browser);
  }
}

bool FubukiClient::DoClose(CefRefPtr<CefBrowser>) {
  return false;
}

void FubukiClient::OnBeforeClose(CefRefPtr<CefBrowser>) {}

void FubukiClient::OnLoadingStateChange(CefRefPtr<CefBrowser>,
                                        bool isLoading,
                                        bool canGoBack,
                                        bool canGoForward) {
  if (!isUi_ && window_) {
    window_->OnTabLoadingState(tabId_, isLoading, canGoBack, canGoForward);
  }
}

void FubukiClient::OnLoadStart(CefRefPtr<CefBrowser>, CefRefPtr<CefFrame> frame, TransitionType) {
  if (!isUi_ && window_ && frame->IsMain()) {
    window_->OnNavigationStarted(tabId_);
  }
}

void FubukiClient::OnLoadEnd(CefRefPtr<CefBrowser>, CefRefPtr<CefFrame> frame, int) {
  if (!isUi_ && window_ && frame->IsMain()) {
    window_->OnNavigationFinished(tabId_);
  }
}

void FubukiClient::OnLoadError(CefRefPtr<CefBrowser>,
                               CefRefPtr<CefFrame> frame,
                               ErrorCode errorCode,
                               const CefString& errorText,
                               const CefString& failedUrl) {
  if (!isUi_ && window_ && frame->IsMain() && errorCode != ERR_ABORTED) {
    const std::string message = errorText.ToString();
    const std::string failed = failedUrl.ToString();
    window_->OnNavigationFailed(tabId_, message);
    const std::string html =
        "<!doctype html><meta charset=\"utf-8\"><title>Page load failed</title>"
        "<style>body{font:15px -apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;margin:48px;color:#202124}"
        "main{max-width:760px}h1{font-size:28px;margin:0 0 12px}p{line-height:1.5;color:#4b5563}"
        "code{background:#f1f3f4;padding:2px 5px;border-radius:4px;word-break:break-all}"
        ".actions{display:flex;gap:8px;margin-top:20px}button,a{border:1px solid #c6ccd4;border-radius:6px;background:#fff;color:#1f2328;"
        "font:inherit;padding:7px 12px;text-decoration:none}</style>"
        "<main><h1>Page load failed</h1><p>" +
        HtmlEscape(message) + "</p><p>Check the URL, reload the page, or go back.</p><p><code>" + HtmlEscape(failed) +
        "</code></p><div class=\"actions\"><a href=\"" + HtmlEscape(failed) +
        "\">Reload</a><button onclick=\"history.back()\">Back</button><a href=\"fubuki://newtab/\">New tab</a></div></main>";
    frame->LoadURL("data:text/html;charset=utf-8," + CefURIEncode(html, false).ToString());
  }
}

void FubukiClient::OnTitleChange(CefRefPtr<CefBrowser>, const CefString& title) {
  if (!isUi_ && window_) {
    window_->OnTabTitle(tabId_, title.ToString());
  }
}

void FubukiClient::OnAddressChange(CefRefPtr<CefBrowser>, CefRefPtr<CefFrame> frame, const CefString& url) {
  if (!isUi_ && window_ && frame->IsMain()) {
    window_->OnTabUrl(tabId_, url.ToString());
  }
}

void FubukiClient::OnFaviconURLChange(CefRefPtr<CefBrowser>, const std::vector<CefString>& icon_urls) {
  if (!isUi_ && window_ && !icon_urls.empty()) {
    window_->OnTabFavicon(tabId_, icon_urls.front().ToString());
  }
}

bool FubukiClient::OnBeforeDownload(CefRefPtr<CefBrowser>,
                                    CefRefPtr<CefDownloadItem> download_item,
                                    const CefString& suggested_name,
                                    CefRefPtr<CefBeforeDownloadCallback> callback) {
  if (!window_ || !download_item || !callback) {
    return false;
  }
  const std::string path = window_->DownloadPathFor(suggested_name.ToString());
  const bool askBeforeDownload = window_->Store().Settings()->GetString("askBeforeDownload") == "on";
  window_->OnDownloadStarted(download_item->GetURL().ToString(), path);
  callback->Continue(path, askBeforeDownload);
  return true;
}

void FubukiClient::OnDownloadUpdated(CefRefPtr<CefBrowser>,
                                     CefRefPtr<CefDownloadItem> download_item,
                                     CefRefPtr<CefDownloadItemCallback>) {
  if (!window_ || !download_item) {
    return;
  }
  std::string state = "in_progress";
  if (download_item->IsComplete()) {
    state = "completed";
  } else if (download_item->IsCanceled()) {
    state = "canceled";
  } else if (download_item->IsInterrupted()) {
    state = "failed";
  }
  window_->OnDownloadUpdated(download_item->GetURL().ToString(),
                             download_item->GetFullPath().ToString(),
                             state,
                             download_item->GetPercentComplete());
}

bool FubukiClient::OnPreKeyEvent(CefRefPtr<CefBrowser>,
                                 const CefKeyEvent& event,
                                 CefEventHandle,
                                 bool* is_keyboard_shortcut) {
  if (!window_ || event.type != KEYEVENT_RAWKEYDOWN) {
    return false;
  }
  const bool commandDown = (event.modifiers & EVENTFLAG_COMMAND_DOWN) != 0;
  const bool altDown = (event.modifiers & EVENTFLAG_ALT_DOWN) != 0;
  const char character = static_cast<char>(event.unmodified_character);
  const bool handled = window_->HandleShortcut(commandDown, altDown, event.windows_key_code, character);
  if (handled && is_keyboard_shortcut) {
    *is_keyboard_shortcut = true;
  }
  return handled;
}

bool FubukiClient::OnBeforeBrowse(CefRefPtr<CefBrowser>,
                                  CefRefPtr<CefFrame> frame,
                                  CefRefPtr<CefRequest> request,
                                  bool user_gesture,
                                  bool is_redirect) {
  if (!window_ || isUi_ || !frame || !frame->IsMain() || !request) {
    return false;
  }
  const std::string url = request->GetURL().ToString();
  if (StartsWith(url, "fubuki://settings/set")) {
    if (!user_gesture || is_redirect || !IsTrustedSettingsActionSource(frame->GetURL().ToString())) {
      return true;
    }
    window_->HandleSettingsUrl(tabId_, url);
    return true;
  }
  if (StartsWith(url, "fubuki://newtab/search")) {
    window_->HandleNewTabSearchUrl(tabId_, url);
    return true;
  }
  return false;
}

void FubukiClient::OnDraggableRegionsChanged(CefRefPtr<CefBrowser>,
                                             CefRefPtr<CefFrame>,
                                             const std::vector<CefDraggableRegion>& regions) {
  if (isUi_ && window_) {
    window_->OnUiDraggableRegionsChanged(regions);
  }
}

bool FubukiClient::OnShowPermissionPrompt(CefRefPtr<CefBrowser>,
                                          uint64_t,
                                          const CefString& requesting_origin,
                                          uint32_t requested_permissions,
                                          CefRefPtr<CefPermissionPromptCallback> callback) {
  if (!callback) {
    return false;
  }

  if (window_ && !window_->IsPrivate()) {
    window_->Store().Log("info",
                         "Permission denied for " + requesting_origin.ToString() + " (" +
                             std::to_string(requested_permissions) + ")");
  }
  callback->Continue(CEF_PERMISSION_RESULT_DENY);
  return true;
}

}  // namespace fubuki
