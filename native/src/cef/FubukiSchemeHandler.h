#pragma once

#include <chrono>
#include <list>
#include <mutex>
#include <string>
#include <unordered_map>

#include "include/cef_resource_handler.h"
#include "include/cef_scheme.h"

namespace fubuki {

class FrostBridge;

class FubukiSchemeHandler : public CefResourceHandler {
public:
  FubukiSchemeHandler(std::string uiDistPath, FrostBridge *engine,
                      bool privateRuntime);

  bool Open(CefRefPtr<CefRequest> request, bool &handle_request,
            CefRefPtr<CefCallback> callback) override;
  void GetResponseHeaders(CefRefPtr<CefResponse> response,
                          int64_t &response_length,
                          CefString &redirectUrl) override;
  bool Read(void *data_out, int bytes_to_read, int &bytes_read,
            CefRefPtr<CefResourceReadCallback> callback) override;
  void Cancel() override;

private:
  bool LoadRequest(const std::string &url);
  bool LoadFile(const std::string &path, const std::string &mimeType);
  void LoadText(std::string body, std::string mimeType, int status);
  std::string ResolveAppPath(const std::string &url) const;

  std::string uiDistPath_;
  // FrostBridge lifetime is owned by BrowserAppController and outlives CEF
  // handlers. This token is immutable and safe to use from the scheme
  // handler's request thread; never retain or inspect BrowserWindow here.
  FrostBridge *engine_ = nullptr;
  bool privateRuntime_ = false;
  std::string data_;
  std::string mimeType_ = "text/plain";
  size_t offset_ = 0;
  int status_ = 200;

  IMPLEMENT_REFCOUNTING(FubukiSchemeHandler);
};

struct CachedPage {
  std::string html;
  std::chrono::steady_clock::time_point expiresAt;
};

class PageCache {
public:
  static PageCache &Instance();

  bool Get(const std::string &url, std::string &html);
  void Set(const std::string &url, std::string html,
           std::chrono::seconds ttl = std::chrono::seconds{5});
  void Invalidate(const std::string &prefix);

private:
  static constexpr size_t kMaxEntries = 32;
  mutable std::mutex mutex_;
  std::list<std::pair<std::string, std::string>> order_;
  std::unordered_map<std::string,
                     std::pair<CachedPage, decltype(order_.begin())>>
      cache_;
};

class FubukiSchemeHandlerFactory : public CefSchemeHandlerFactory {
public:
  explicit FubukiSchemeHandlerFactory(std::string uiDistPath,
                                      FrostBridge *engine = nullptr,
                                      bool privateRuntime = false);
  CefRefPtr<CefResourceHandler> Create(CefRefPtr<CefBrowser> browser,
                                       CefRefPtr<CefFrame> frame,
                                       const CefString &scheme_name,
                                       CefRefPtr<CefRequest> request) override;

private:
  std::string uiDistPath_;
  FrostBridge *engine_ = nullptr;
  bool privateRuntime_ = false;
  IMPLEMENT_REFCOUNTING(FubukiSchemeHandlerFactory);
};

}  // namespace fubuki
