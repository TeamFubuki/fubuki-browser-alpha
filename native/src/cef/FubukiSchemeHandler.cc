#include "cef/FubukiSchemeHandler.h"

#include <fstream>
#include <sstream>
#include <cstring>

namespace fubuki {

namespace {

std::string MimeForPath(const std::string& path) {
  if (path.ends_with(".html")) return "text/html";
  if (path.ends_with(".js")) return "application/javascript";
  if (path.ends_with(".css")) return "text/css";
  if (path.ends_with(".svg")) return "image/svg+xml";
  if (path.ends_with(".json")) return "application/json";
  if (path.ends_with(".png")) return "image/png";
  if (path.ends_with(".ico")) return "image/x-icon";
  return "application/octet-stream";
}

std::string NewTabHtml() {
  return R"(<!doctype html>
<html><head><meta charset="utf-8"><title>New Tab</title>
<style>
body{margin:0;font:15px -apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;background:#f6f7f8;color:#202124;display:grid;place-items:center;height:100vh}
main{width:min(640px,calc(100vw - 48px))}
h1{font-size:28px;font-weight:650;margin:0 0 10px}
p{color:#5f6368;margin:0}
</style></head><body><main><h1>Fubuki Browser Alpha</h1><p>Enter a URL or search query in the address bar.</p></main></body></html>)";
}

}  // namespace

FubukiSchemeHandler::FubukiSchemeHandler(std::string uiDistPath) : uiDistPath_(std::move(uiDistPath)) {}

bool FubukiSchemeHandler::Open(CefRefPtr<CefRequest> request, bool& handle_request, CefRefPtr<CefCallback>) {
  handle_request = true;
  return LoadRequest(request->GetURL().ToString());
}

void FubukiSchemeHandler::GetResponseHeaders(CefRefPtr<CefResponse> response,
                                             int64_t& response_length,
                                             CefString&) {
  response->SetStatus(status_);
  response->SetMimeType(mimeType_);
  CefResponse::HeaderMap headers;
  if (mimeType_.rfind("text/", 0) == 0 || mimeType_ == "application/javascript" || mimeType_ == "application/json") {
    headers.insert({"Content-Type", mimeType_ + "; charset=utf-8"});
  } else {
    headers.insert({"Content-Type", mimeType_});
  }
  headers.insert({"Cache-Control", "no-store, max-age=0"});
  response->SetHeaderMap(headers);
  response_length = static_cast<int64_t>(data_.size());
}

bool FubukiSchemeHandler::Read(void* data_out, int bytes_to_read, int& bytes_read, CefRefPtr<CefResourceReadCallback>) {
  const size_t remaining = data_.size() - offset_;
  const size_t count = std::min<size_t>(remaining, static_cast<size_t>(bytes_to_read));
  if (count > 0) {
    std::memcpy(data_out, data_.data() + offset_, count);
    offset_ += count;
    bytes_read = static_cast<int>(count);
    return true;
  }
  bytes_read = 0;
  return false;
}

void FubukiSchemeHandler::Cancel() {}

bool FubukiSchemeHandler::LoadRequest(const std::string& url) {
  offset_ = 0;
  if (url.rfind("fubuki://newtab/", 0) == 0) {
    LoadText(NewTabHtml(), "text/html", 200);
    return true;
  }
  if (url.rfind("fubuki://app/", 0) == 0) {
    const std::string path = ResolveAppPath(url);
    if (LoadFile(path, MimeForPath(path))) {
      return true;
    }
    LoadText("Fubuki UI build not found. Run `npm install && npm run build` in ui/.", "text/plain", 404);
    return true;
  }
  LoadText("Not found", "text/plain", 404);
  return true;
}

bool FubukiSchemeHandler::LoadFile(const std::string& path, const std::string& mimeType) {
  std::ifstream file(path, std::ios::binary);
  if (!file) {
    return false;
  }
  std::ostringstream buffer;
  buffer << file.rdbuf();
  LoadText(buffer.str(), mimeType, 200);
  return true;
}

void FubukiSchemeHandler::LoadText(std::string body, std::string mimeType, int status) {
  data_ = std::move(body);
  mimeType_ = std::move(mimeType);
  status_ = status;
}

std::string FubukiSchemeHandler::ResolveAppPath(const std::string& url) const {
  std::string path = url.substr(std::string("fubuki://app/").size());
  const size_t query = path.find_first_of("?#");
  if (query != std::string::npos) {
    path = path.substr(0, query);
  }
  if (path.empty() || path == "/") {
    path = "index.html";
  }
  if (path.find("..") != std::string::npos) {
    path = "index.html";
  }
  return uiDistPath_ + "/" + path;
}

FubukiSchemeHandlerFactory::FubukiSchemeHandlerFactory(std::string uiDistPath)
    : uiDistPath_(std::move(uiDistPath)) {}

CefRefPtr<CefResourceHandler> FubukiSchemeHandlerFactory::Create(CefRefPtr<CefBrowser>,
                                                                 CefRefPtr<CefFrame>,
                                                                 const CefString&,
                                                                 CefRefPtr<CefRequest>) {
  return new FubukiSchemeHandler(uiDistPath_);
}

}  // namespace fubuki
