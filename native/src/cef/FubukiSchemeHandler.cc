#include "cef/FubukiSchemeHandler.h"

#include <sqlite3.h>

#include <algorithm>
#include <cstdlib>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <sstream>
#include <unordered_map>
#include <vector>

#include "browser/BrowserAppController.h"
#include "browser/BrowserWindow.h"
#include "include/cef_parser.h"

namespace fubuki {

namespace {

struct Record {
  std::string title;
  std::string url;
  std::string faviconUrl;
  std::string path;
  std::string state;
  int percent = 0;
  std::string createdAt;
  std::string downloadId;
};

struct PermissionRecord {
  std::string origin;
  std::string permission;
  std::string value;
  std::string createdAt;
};

std::filesystem::path ProfilePath() {
  const char* home = std::getenv("HOME");
  return home ? std::filesystem::path(home) / "Library/Application Support/Fubuki Browser Alpha"
              : std::filesystem::temp_directory_path() / "Fubuki Browser Alpha";
}

std::filesystem::path DatabasePath() {
  // Internal pages are read-only projections of the FrostEngine store. The
  // legacy fubuki.sqlite3 database split reads from writes, making successful
  // setting/delete actions appear to do nothing.
  return ProfilePath() / "frost-engine.sqlite3";
}

std::string MimeForPath(const std::string& path) {
  if (path.ends_with(".html"))
    return "text/html";
  if (path.ends_with(".js"))
    return "application/javascript";
  if (path.ends_with(".css"))
    return "text/css";
  if (path.ends_with(".svg"))
    return "image/svg+xml";
  if (path.ends_with(".json"))
    return "application/json";
  if (path.ends_with(".png"))
    return "image/png";
  if (path.ends_with(".ico"))
    return "image/x-icon";
  return "application/octet-stream";
}

std::string ColumnText(sqlite3_stmt* statement, int column) {
  const unsigned char* text = sqlite3_column_text(statement, column);
  return text ? reinterpret_cast<const char*>(text) : "";
}

sqlite3* OpenDatabase() {
  // The Rust store owns creation and migrations. Internal pages only read the
  // database so they cannot race a writer with their own DDL.
  static sqlite3* cached = nullptr;
  if (cached) {
    return cached;
  }
  if (sqlite3_open_v2(DatabasePath().string().c_str(), &cached,
                      SQLITE_OPEN_READONLY | SQLITE_OPEN_FULLMUTEX, nullptr) != SQLITE_OK) {
    if (cached) {
      sqlite3_close(cached);
    }
    cached = nullptr;
    return nullptr;
  }
  if (sqlite3_busy_timeout(cached, 2500) != SQLITE_OK) {
    sqlite3_close(cached);
    cached = nullptr;
    return nullptr;
  }
  return cached;
}

std::string Setting(const std::string& key, const std::string& fallback = "") {
  sqlite3* db = OpenDatabase();
  if (!db)
    return fallback;
  sqlite3_stmt* statement = nullptr;
  if (sqlite3_prepare_v2(db, "SELECT value FROM settings WHERE key=?", -1, &statement, nullptr) !=
      SQLITE_OK) {
    return fallback;
  }
  if (sqlite3_bind_text(statement, 1, key.c_str(), static_cast<int>(key.size()),
                        SQLITE_TRANSIENT) != SQLITE_OK) {
    sqlite3_finalize(statement);
    return fallback;
  }
  std::string value = fallback;
  if (sqlite3_step(statement) == SQLITE_ROW) {
    value = ColumnText(statement, 0);
  }
  sqlite3_finalize(statement);
  return value.empty() ? fallback : value;
}

std::string BrowserAppearance() {
  const std::string appearance = Setting("appearance", "system");
  if (appearance == "light" || appearance == "dark") {
    return appearance;
  }
  return "system";
}

std::string BrowserLanguage() {
  const std::string setting = Setting("language", "system");
  if (setting == "ja" || setting == "en") {
    return setting;
  }
  const char* lang = std::getenv("LANG");
  return lang && std::string(lang).rfind("ja", 0) == 0 ? "ja" : "en";
}

enum class DatabaseErrorKind {
  kNone,
  kBusy,
  kOpenFailed,
  kPrepareFailed,
  kBindFailed,
  kStepFailed
};

template <typename T>
struct DatabaseResult {
  T value{};
  DatabaseErrorKind error = DatabaseErrorKind::kNone;

  bool Ok() const {
    return error == DatabaseErrorKind::kNone;
  }
};

DatabaseErrorKind DatabaseErrorFor(int code, DatabaseErrorKind fallback) {
  return code == SQLITE_BUSY || code == SQLITE_LOCKED ? DatabaseErrorKind::kBusy : fallback;
}

DatabaseResult<std::vector<Record>> QueryRecords(const std::string& table, int limit) {
  DatabaseResult<std::vector<Record>> result;
  sqlite3* db = OpenDatabase();
  if (!db) {
    result.error = DatabaseErrorKind::kOpenFailed;
    return result;
  }

  const std::string sql =
      table == "bookmarks" ? "SELECT title,url,favicon_url,'','',0,created_at,'' "
                             "FROM bookmarks ORDER BY id DESC LIMIT ?"
      : table == "history" ? "SELECT title,url,favicon_url,'','',0,created_at,'' FROM "
                             "history ORDER BY id DESC LIMIT ?"
      : table == "logs"    ? "SELECT message,'','',level,'',0,created_at,'' FROM logs ORDER BY id "
                             "DESC LIMIT ?"
                           : "SELECT "
                             "'',url,'',path,state,percent,COALESCE(updated_at,created_at),"
                             "download_id FROM "
                             "downloads ORDER BY COALESCE(updated_at,created_at) DESC,id DESC "
                             "LIMIT ?";
  sqlite3_stmt* statement = nullptr;
  int rc = sqlite3_prepare_v2(db, sql.c_str(), -1, &statement, nullptr);
  if (rc != SQLITE_OK) {
    result.error = DatabaseErrorFor(rc, DatabaseErrorKind::kPrepareFailed);
    return result;
  }
  rc = sqlite3_bind_int(statement, 1, limit);
  if (rc != SQLITE_OK) {
    sqlite3_finalize(statement);
    result.error = DatabaseErrorFor(rc, DatabaseErrorKind::kBindFailed);
    return result;
  }

  while ((rc = sqlite3_step(statement)) == SQLITE_ROW) {
    Record record;
    record.title = ColumnText(statement, 0);
    record.url = ColumnText(statement, 1);
    record.faviconUrl = ColumnText(statement, 2);
    record.path = ColumnText(statement, 3);
    record.state = ColumnText(statement, 4);
    record.percent = sqlite3_column_int(statement, 5);
    record.createdAt = ColumnText(statement, 6);
    record.downloadId = ColumnText(statement, 7);
    result.value.push_back(record);
  }

  sqlite3_finalize(statement);
  if (rc != SQLITE_DONE) {
    result.value.clear();
    result.error = DatabaseErrorFor(rc, DatabaseErrorKind::kStepFailed);
  }
  return result;
}

DatabaseResult<std::vector<PermissionRecord>> QueryPermissions() {
  DatabaseResult<std::vector<PermissionRecord>> result;
  sqlite3* db = OpenDatabase();
  if (!db) {
    result.error = DatabaseErrorKind::kOpenFailed;
    return result;
  }
  sqlite3_stmt* statement = nullptr;
  const int rc = sqlite3_prepare_v2(
      db, "SELECT origin,permission,value,created_at FROM permissions ORDER BY id DESC LIMIT 200",
      -1, &statement, nullptr);
  if (rc != SQLITE_OK) {
    result.error = DatabaseErrorFor(rc, DatabaseErrorKind::kPrepareFailed);
    return result;
  }
  int step = SQLITE_OK;
  while ((step = sqlite3_step(statement)) == SQLITE_ROW) {
    result.value.push_back({ColumnText(statement, 0), ColumnText(statement, 1),
                            ColumnText(statement, 2), ColumnText(statement, 3)});
  }
  sqlite3_finalize(statement);
  if (step != SQLITE_DONE) {
    result.value.clear();
    result.error = DatabaseErrorFor(step, DatabaseErrorKind::kStepFailed);
  }
  return result;
}

struct InternalPageData {
  std::string json;
  bool ok = true;
};

InternalPageData ReadInternalPageData(const std::string& page) {
  auto root = CefDictionaryValue::Create();
  root->SetString("language", BrowserLanguage());
  root->SetString("appearance", BrowserAppearance());
  auto settings = CefDictionaryValue::Create();
  // Only expose the settings used by internal pages.
  for (const auto* key : {"appearance", "language", "startupBehavior", "homeUrl", "newTabPage",
                          "defaultZoomLevel", "sidebarVisible", "sidebarWidth", "searchEngine",
                          "customSearchUrl", "askBeforeDownload", "downloadDirectory"}) {
    settings->SetString(key, Setting(key));
  }
  root->SetDictionary("settings", settings);
  bool ok = true;
  auto records = CefListValue::Create();
  if (page == "bookmarks" || page == "history" || page == "downloads" || page == "debug") {
    const auto query = QueryRecords(page == "debug" ? "logs" : page, page == "downloads" ? 50
                                                                     : page == "debug"   ? 80
                                                                                         : 500);
    ok = query.Ok();
    for (const auto& record : query.value) {
      auto item = CefDictionaryValue::Create();
      item->SetString("title", record.title);
      item->SetString("url", record.url);
      item->SetString("faviconUrl", record.faviconUrl);
      item->SetString("path", record.path);
      item->SetString("state", record.state);
      item->SetInt("percent", record.percent);
      item->SetString("createdAt", record.createdAt);
      item->SetString("downloadId", record.downloadId);
      records->SetDictionary(records->GetSize(), item);
    }
  }
  root->SetList("records", records);
  auto permissions = CefListValue::Create();
  if (page == "settings") {
    const auto query = QueryPermissions();
    ok = query.Ok();
    for (const auto& permission : query.value) {
      auto item = CefDictionaryValue::Create();
      item->SetString("origin", permission.origin);
      item->SetString("permission", permission.permission);
      item->SetString("value", permission.value);
      item->SetString("createdAt", permission.createdAt);
      permissions->SetDictionary(permissions->GetSize(), item);
    }
  }
  root->SetList("permissions", permissions);
  if (page == "debug") {
    root->SetString("profilePath", ProfilePath().string());
    // Diagnostics are host observations; browser state remains engine-owned.
    auto windows = CefListValue::Create();
    auto commands = CefListValue::Create();
    auto events = CefListValue::Create();
    if (auto* app = GetBrowserAppController()) {
      for (auto* window : app->Windows()) {
        auto item = CefDictionaryValue::Create();
        item->SetString("id", window->WindowId());
        item->SetBool("isPrivate", window->IsPrivate());
        auto tabs = CefListValue::Create();
        for (const auto& tab : window->Tabs().GetTabs()) {
          auto value = CefDictionaryValue::Create();
          value->SetString("title", tab.title.empty() ? tab.url : tab.title);
          value->SetBool("isActive", tab.isActive);
          tabs->SetDictionary(tabs->GetSize(), value);
        }
        item->SetList("tabs", tabs);
        windows->SetDictionary(windows->GetSize(), item);
      }
      if (auto* active = app->ActiveWindow())
        commands = active->Commands().List();
      for (const auto& event : app->Events().RecentEvents()) {
        auto item = CefDictionaryValue::Create();
        item->SetString("name", event.name);
        item->SetString("message", event.windowId + " " + event.tabId + " " + event.message);
        events->SetDictionary(events->GetSize(), item);
      }
    }
    root->SetList("windows", windows);
    root->SetList("commands", commands);
    root->SetList("events", events);
  }
  auto value = CefValue::Create();
  value->SetDictionary(root);
  return {CefWriteJSON(value, JSON_WRITER_DEFAULT).ToString(), ok};
}

}  // namespace

// PageCache implementation

PageCache& PageCache::Instance() {
  static PageCache instance;
  return instance;
}

bool PageCache::Get(const std::string& url, std::string& html) {
  std::lock_guard<std::mutex> lock(mutex_);
  auto it = cache_.find(url);
  if (it == cache_.end()) {
    return false;
  }
  if (std::chrono::steady_clock::now() > it->second.first.expiresAt) {
    order_.erase(it->second.second);
    cache_.erase(it);
    return false;
  }
  order_.splice(order_.begin(), order_, it->second.second);
  html = it->second.first.html;
  return true;
}

void PageCache::Set(const std::string& url, std::string html, std::chrono::seconds ttl) {
  std::lock_guard<std::mutex> lock(mutex_);
  auto it = cache_.find(url);
  if (it != cache_.end()) {
    order_.erase(it->second.second);
    cache_.erase(it);
  }
  if (cache_.size() >= kMaxEntries) {
    auto last = std::prev(order_.end());
    cache_.erase(last->first);
    order_.erase(last);
  }
  order_.emplace_front(url, html);
  cache_[url] = {{std::move(html), std::chrono::steady_clock::now() + ttl}, order_.begin()};
}

void PageCache::Invalidate(const std::string& prefix) {
  std::lock_guard<std::mutex> lock(mutex_);
  for (auto it = order_.begin(); it != order_.end();) {
    if (it->first.find(prefix) == 0) {
      cache_.erase(it->first);
      it = order_.erase(it);
    } else {
      ++it;
    }
  }
}

FubukiSchemeHandler::FubukiSchemeHandler(std::string uiDistPath)
    : uiDistPath_(std::move(uiDistPath)) {}

bool FubukiSchemeHandler::Open(CefRefPtr<CefRequest> request, bool& handle_request,
                               CefRefPtr<CefCallback>) {
  handle_request = true;
  return LoadRequest(request->GetURL().ToString());
}

void FubukiSchemeHandler::GetResponseHeaders(CefRefPtr<CefResponse> response,
                                             int64_t& response_length, CefString& redirectUrl) {
  redirectUrl = redirectUrl_;
  response->SetStatus(status_);
  response->SetMimeType(mimeType_);
  CefResponse::HeaderMap headers;
  headers.insert(
      {"Content-Type", mimeType_ + (mimeType_.rfind("text/", 0) == 0 ? "; charset=utf-8" : "")});
  headers.insert({"Cache-Control", "no-store, max-age=0"});
  response->SetHeaderMap(headers);
  response_length = static_cast<int64_t>(data_.size());
}

bool FubukiSchemeHandler::Read(void* data_out, int bytes_to_read, int& bytes_read,
                               CefRefPtr<CefResourceReadCallback>) {
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

std::string ExtractQueryParam(const std::string& url, const std::string& key) {
  const size_t qpos = url.find('?');
  if (qpos == std::string::npos)
    return "";
  const std::string query = url.substr(qpos + 1);
  const std::string needle = key + "=";
  size_t start = 0;
  while (start < query.size()) {
    const size_t pos = query.find(needle, start);
    if (pos == std::string::npos)
      return "";
    if (pos == 0 || query[pos - 1] == '&') {
      const size_t valueStart = pos + needle.size();
      const size_t ampersand = query.find('&', valueStart);
      return query.substr(
          valueStart, ampersand == std::string::npos ? std::string::npos : ampersand - valueStart);
    }
    start = pos + 1;
  }
  return "";
}

std::string SearchRedirectUrl(const std::string& query) {
  if (query.empty())
    return "";
  const std::string engine = Setting("searchEngine", "google");
  const std::string customUrl =
      Setting("customSearchUrl", "https://www.google.com/search?q={query}");
  std::string encoded = CefURIEncode(query, false).ToString();
  if (engine == "duckduckgo")
    return "https://duckduckgo.com/?q=" + encoded;
  if (engine == "bing")
    return "https://www.bing.com/search?q=" + encoded;
  if (engine == "custom") {
    std::string url = customUrl;
    const size_t pos = url.find("{query}");
    if (pos != std::string::npos)
      url.replace(pos, 7, encoded);
    return url;
  }
  return "https://www.google.com/search?q=" + encoded;
}

bool FubukiSchemeHandler::LoadRequest(const std::string& url) {
  offset_ = 0;
  redirectUrl_.clear();
  auto& cache = PageCache::Instance();

  // State-changing internal-page actions are intercepted by FubukiClient only
  // after it verifies the HTTP method, user gesture where required, and trusted
  // source page. The scheme handler itself never executes a mutation, so direct
  // fubuki://settings/set navigation cannot become a destructive GET endpoint.
  if (url.rfind("fubuki://settings/set", 0) == 0) {
    LoadText("Action rejected. Settings changes must be submitted through the UI.", "text/plain",
             403);
    return true;
  }

  // Handle new tab search: fubuki://newtab/search?q=...
  if (url.rfind("fubuki://newtab/search", 0) == 0) {
    const std::string query = ExtractQueryParam(url, "q");
    const std::string redirect = SearchRedirectUrl(query);
    if (!redirect.empty()) {
      redirectUrl_ = redirect;
      LoadText("", "text/plain", 302);
      return true;
    }
    // Empty query — just show new tab
  }

  CefURLParts parts;
  if (CefParseURL(url, parts)) {
    const std::string host = CefString(&parts.host).ToString();
    const bool internal = host == "newtab" || host == "bookmarks" || host == "downloads" ||
                          host == "history" || host == "settings" || host == "debug";
    if (internal) {
      const std::string path = host == "newtab" && CefString(&parts.path).ToString() == "/search"
                                   ? "/"
                                   : CefString(&parts.path).ToString();
      if (path == "/data.json") {
        std::string json;
        const std::string cacheKey = "fubuki://" + host + "/data.json";
        if (host != "debug" && cache.Get(cacheKey, json)) {
          LoadText(std::move(json), "application/json", 200);
        } else {
          auto data = ReadInternalPageData(host);
          if (data.ok && host != "debug")
            cache.Set(cacheKey, data.json, std::chrono::seconds{2});
          LoadText(std::move(data.json), "application/json", data.ok ? 200 : 503);
        }
        return true;
      }
      const std::string relative = path.empty() || path == "/" ? "index.html" : path.substr(1);
      // Serve only exact page roots and built assets. Never fall back to HTML for a missing asset.
      if (relative.find("..") == std::string::npos && relative.find('\\') == std::string::npos &&
          (relative == "index.html" || relative == "logo.svg" ||
           relative.rfind("assets/", 0) == 0) &&
          LoadFile(std::string(FUBUKI_INTERNAL_PAGES_DIST) + "/" + relative,
                   MimeForPath(relative))) {
        return true;
      }
      LoadText("Internal page not found. Run make internal-pages.", "text/plain", 404);
      return true;
    }
  }
  if (url.rfind("fubuki://app/", 0) == 0) {
    const std::string path = ResolveAppPath(url);
    if (LoadFile(path, MimeForPath(path))) {
      return true;
    }
    LoadText("Fubuki UI build not found. Run `pnpm build` in ui/.", "text/plain", 404);
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
  if (path.empty() || path == "/" || path.find("..") != std::string::npos) {
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
