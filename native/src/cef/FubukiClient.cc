#include "cef/FubukiClient.h"

#include "browser/BrowserWindow.h"
#include "include/cef_parser.h"
#include "include/wrapper/cef_helpers.h"

namespace fubuki {

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
    window_->OnNavigationFailed(tabId_, message);
    const std::string html =
        "<!doctype html><meta charset=\"utf-8\"><title>Page load failed</title>"
        "<style>body{font:15px -apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;margin:48px;color:#202124}"
        "main{max-width:720px}h1{font-size:28px}code{background:#f1f3f4;padding:2px 5px;border-radius:4px}</style>"
        "<main><h1>Page load failed</h1><p>" +
        message + "</p><p><code>" + failedUrl.ToString() + "</code></p></main>";
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
  window_->OnDownloadStarted(download_item->GetURL().ToString(), path);
  callback->Continue(path, false);
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
    state = "complete";
  } else if (download_item->IsCanceled()) {
    state = "canceled";
  }
  window_->OnDownloadUpdated(download_item->GetURL().ToString(),
                             download_item->GetFullPath().ToString(),
                             state,
                             download_item->GetPercentComplete());
}

}  // namespace fubuki
