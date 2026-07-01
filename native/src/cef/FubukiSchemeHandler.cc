#include "cef/FubukiSchemeHandler.h"

#include <algorithm>
#include <cstdlib>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <sstream>
#include <vector>

#include <sqlite3.h>

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
};

std::filesystem::path ProfilePath() {
  const char* home = std::getenv("HOME");
  return home ? std::filesystem::path(home) / "Library/Application Support/Fubuki Browser Alpha"
              : std::filesystem::temp_directory_path() / "Fubuki Browser Alpha";
}

std::filesystem::path DatabasePath() {
  return ProfilePath() / "fubuki.sqlite3";
}

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

std::string HtmlEscape(const std::string& value) {
  std::ostringstream out;
  for (const char c : value) {
    switch (c) {
      case '&': out << "&amp;"; break;
      case '<': out << "&lt;"; break;
      case '>': out << "&gt;"; break;
      case '"': out << "&quot;"; break;
      case '\'': out << "&#39;"; break;
      default: out << c; break;
    }
  }
  return out.str();
}

void Execute(sqlite3* db, const std::string& sql) {
  sqlite3_exec(db, sql.c_str(), nullptr, nullptr, nullptr);
}

std::string ColumnText(sqlite3_stmt* statement, int column) {
  const unsigned char* text = sqlite3_column_text(statement, column);
  return text ? reinterpret_cast<const char*>(text) : "";
}

sqlite3* OpenDatabase() {
  std::filesystem::create_directories(ProfilePath());
  sqlite3* db = nullptr;
  if (sqlite3_open(DatabasePath().string().c_str(), &db) != SQLITE_OK) {
    return nullptr;
  }
  Execute(db, "CREATE TABLE IF NOT EXISTS settings(key TEXT PRIMARY KEY, value TEXT NOT NULL)");
  Execute(db, "CREATE TABLE IF NOT EXISTS bookmarks(id INTEGER PRIMARY KEY AUTOINCREMENT,title TEXT NOT NULL,url TEXT NOT NULL UNIQUE,favicon_url TEXT,created_at TEXT NOT NULL)");
  Execute(db, "CREATE TABLE IF NOT EXISTS history(id INTEGER PRIMARY KEY AUTOINCREMENT,title TEXT NOT NULL,url TEXT NOT NULL,created_at TEXT NOT NULL)");
  Execute(db, "CREATE TABLE IF NOT EXISTS downloads(id INTEGER PRIMARY KEY AUTOINCREMENT,url TEXT,path TEXT,state TEXT,percent INTEGER DEFAULT 0,created_at TEXT NOT NULL,updated_at TEXT)");
  Execute(db, "ALTER TABLE downloads ADD COLUMN updated_at TEXT");
  Execute(db, "UPDATE downloads SET updated_at=created_at WHERE updated_at IS NULL OR updated_at=''");
  return db;
}

std::string Setting(const std::string& key, const std::string& fallback = "") {
  sqlite3* db = OpenDatabase();
  if (!db) return fallback;
  sqlite3_stmt* statement = nullptr;
  sqlite3_prepare_v2(db, "SELECT value FROM settings WHERE key=?", -1, &statement, nullptr);
  sqlite3_bind_text(statement, 1, key.c_str(), static_cast<int>(key.size()), SQLITE_TRANSIENT);
  std::string value = fallback;
  if (sqlite3_step(statement) == SQLITE_ROW) {
    value = ColumnText(statement, 0);
  }
  sqlite3_finalize(statement);
  sqlite3_close(db);
  return value.empty() ? fallback : value;
}

std::vector<Record> QueryRecords(const std::string& table, int limit) {
  sqlite3* db = OpenDatabase();
  if (!db) return {};

  const std::string sql = table == "bookmarks"
                              ? "SELECT title,url,favicon_url,'','',0,created_at FROM bookmarks ORDER BY id DESC LIMIT ?"
                          : table == "history"
                              ? "SELECT title,url,'','','',0,created_at FROM history ORDER BY id DESC LIMIT ?"
                              : "SELECT '',url,'',path,state,percent,COALESCE(updated_at,created_at) FROM downloads ORDER BY COALESCE(updated_at,created_at) DESC,id DESC LIMIT ?";
  sqlite3_stmt* statement = nullptr;
  sqlite3_prepare_v2(db, sql.c_str(), -1, &statement, nullptr);
  sqlite3_bind_int(statement, 1, limit);

  std::vector<Record> records;
  while (sqlite3_step(statement) == SQLITE_ROW) {
    Record record;
    record.title = ColumnText(statement, 0);
    record.url = ColumnText(statement, 1);
    record.faviconUrl = ColumnText(statement, 2);
    record.path = ColumnText(statement, 3);
    record.state = ColumnText(statement, 4);
    record.percent = sqlite3_column_int(statement, 5);
    record.createdAt = ColumnText(statement, 6);
    records.push_back(record);
  }

  sqlite3_finalize(statement);
  sqlite3_close(db);
  return records;
}

std::string FubukiLogoSvg(const std::string& className = "logo") {
  return "<svg class=\"" + className +
         "\" width=\"512\" height=\"512\" viewBox=\"0 0 512 512\" fill=\"none\" xmlns=\"http://www.w3.org/2000/svg\">"
         "<path d=\"M128 440L183.252 248.366M470 72L252.28 72C238.617 72 226.68 81.2317 223.244 94.4554L183.252 248.366M183.252 248.366H363.904\" stroke=\"url(#paint0_linear_7_2)\" stroke-width=\"25\" stroke-linecap=\"round\"/>"
         "<path d=\"M95.6021 142.602L148.204 195.204M148.204 195.204L43.0001 195.204M148.204 195.204L95.6021 247.806M148.204 195.204V300.408M148.204 195.204L200.806 247.806M148.204 195.204V90M148.204 195.204L200.806 142.602M148.204 195.204H253.408\" stroke=\"#1AADEB\" stroke-width=\"5\" stroke-linecap=\"round\"/>"
         "<defs><linearGradient id=\"paint0_linear_7_2\" x1=\"257.282\" y1=\"72\" x2=\"257.282\" y2=\"476.326\" gradientUnits=\"userSpaceOnUse\"><stop stop-color=\"#FF9686\"/><stop offset=\"1\" stop-color=\"#A7ABE0\"/></linearGradient></defs>"
         "</svg>";
}

std::string FubukiFaviconLink() {
  return "<link rel=\"icon\" type=\"image/svg+xml\" href=\"data:image/svg+xml," +
         CefURIEncode(FubukiLogoSvg(), false).ToString() + "\">";
}

std::string PageChrome(const std::string& title, const std::string& body) {
  const std::string appearance = Setting("appearance", "system");
  std::ostringstream html;
  html << "<!doctype html><html data-appearance=\"" << HtmlEscape(appearance) << "\"><head><meta charset=\"utf-8\"><title>"
       << HtmlEscape(title) << "</title>" << FubukiFaviconLink() << R"(<style>
*{box-sizing:border-box}body{margin:0;background:var(--bg);color:var(--text);font:14px -apple-system,BlinkMacSystemFont,"SF Pro Text","Helvetica Neue",sans-serif;--bg:#f7f8fa;--surface:#fff;--text:#17191d;--muted:#626b77;--line:rgb(24 32 44/.13);--hover:rgb(24 32 44/.06);--accent:#2563eb}
html[data-appearance=dark] body{--bg:#17191d;--surface:#202329;--text:#f4f6f8;--muted:#a5adba;--line:rgb(255 255 255/.13);--hover:rgb(255 255 255/.07);--accent:#8ab4ff;color-scheme:dark}
@media(prefers-color-scheme:dark){html[data-appearance=system] body{--bg:#17191d;--surface:#202329;--text:#f4f6f8;--muted:#a5adba;--line:rgb(255 255 255/.13);--hover:rgb(255 255 255/.07);--accent:#8ab4ff;color-scheme:dark}}
main{width:min(880px,calc(100vw - 48px));margin:0 auto;padding:36px 0 56px}header{display:flex;align-items:center;gap:12px;margin-bottom:24px}.logo{width:34px;height:34px}h1{font-size:30px;line-height:1.1;margin:0;font-weight:720}a{color:inherit}.list{display:grid;gap:6px}.row{min-height:46px;display:grid;grid-template-columns:22px minmax(0,1fr) auto;align-items:center;gap:10px;padding:8px 10px;border:1px solid var(--line);border-radius:8px;background:var(--surface);text-decoration:none}.row:hover,.button:hover{background:color-mix(in srgb,var(--surface) 88%,var(--accent))}.favicon{width:16px;height:16px;border-radius:4px;background:linear-gradient(135deg,#46a7ff,#6b78e6 58%,#ff9585)}.favicon img{width:16px;height:16px;border-radius:4px}.title{min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;font-weight:620}.meta{min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;color:var(--muted);font-size:12px}.button,.chip{height:30px;display:inline-grid;place-items:center;border:1px solid var(--line);border-radius:7px;padding:0 10px;background:var(--surface);color:var(--text);text-decoration:none;font:inherit;font-weight:620}.danger{color:#b42318}.empty{color:var(--muted);padding:18px 0}.section{display:grid;gap:16px}.field{display:grid;gap:10px;padding:14px;border:1px solid var(--line);border-radius:8px;background:var(--surface)}.field>span{font-weight:650}.segmented{display:flex;flex-wrap:wrap;gap:8px}.selected{border-color:var(--accent);color:var(--accent)}input{height:32px;border:1px solid var(--line);border-radius:7px;padding:0 10px;background:var(--bg);color:var(--text);font:inherit}.inline-form{display:flex;gap:8px;align-items:center;flex-wrap:wrap}
</style></head><body><main><header>)"
       << FubukiLogoSvg() << "<h1>" << HtmlEscape(title) << "</h1></header>" << body << "</main></body></html>";
  return html.str();
}

std::string ActionLink(const std::string& key, const std::string& value, const std::string& returnUrl) {
  return "fubuki://settings/set?key=" + key + "&value=" + CefURIEncode(value, false).ToString() +
         "&return=" + CefURIEncode(returnUrl, false).ToString();
}

std::string FileName(const std::string& path, const std::string& url) {
  const std::string source = path.empty() ? url : path;
  const size_t slash = source.find_last_of("/\\");
  return slash == std::string::npos ? source : source.substr(slash + 1);
}

std::string BookmarksHtml() {
  std::ostringstream body;
  const auto records = QueryRecords("bookmarks", 500);
  if (records.empty()) {
    body << "<p class=\"empty\">No bookmarks</p>";
  } else {
    body << "<div class=\"list\">";
    for (const auto& record : records) {
      body << "<div class=\"row\"><span class=\"favicon\">";
      if (!record.faviconUrl.empty()) {
        body << "<img alt=\"\" src=\"" << HtmlEscape(record.faviconUrl) << "\">";
      }
      body << "</span><a href=\"" << HtmlEscape(record.url) << "\" title=\"" << HtmlEscape(record.url)
           << "\"><div class=\"title\">" << HtmlEscape(record.title.empty() ? record.url : record.title)
           << "</div><div class=\"meta\">" << HtmlEscape(record.url) << "</div></a>"
           << "<a class=\"button danger\" href=\"" << ActionLink("removeBookmark", record.url, "fubuki://bookmarks/")
           << "\">Delete</a></div>";
    }
    body << "</div>";
  }
  return PageChrome("Bookmarks", body.str());
}

std::string DownloadsHtml() {
  std::ostringstream body;
  const auto records = QueryRecords("downloads", 50);
  if (records.empty()) {
    body << "<p class=\"empty\">No downloads</p>";
  } else {
    body << "<div class=\"list\">";
    for (const auto& record : records) {
      body << "<article class=\"row\"><span aria-hidden=\"true\">↓</span><div><div class=\"title\">"
           << HtmlEscape(FileName(record.path, record.url)) << "</div><div class=\"meta\">"
           << HtmlEscape(record.path.empty() ? record.url : record.path) << "</div></div><span class=\"meta\">"
           << HtmlEscape(record.state.empty() ? "unknown" : record.state) << " " << record.percent << "%</span></article>";
    }
    body << "</div>";
  }
  return PageChrome("Downloads", body.str());
}

std::string HistoryHtml() {
  std::ostringstream body;
  const auto records = QueryRecords("history", 500);
  if (records.empty()) {
    body << "<p class=\"empty\">No history</p>";
  } else {
    body << "<div class=\"list\">";
    for (const auto& record : records) {
      body << "<a class=\"row\" href=\"" << HtmlEscape(record.url) << "\" title=\"" << HtmlEscape(record.url)
           << "\"><span class=\"favicon\"></span><span><span class=\"title\">" << HtmlEscape(record.title.empty() ? record.url : record.title)
           << "</span><span class=\"meta\">" << HtmlEscape(record.createdAt + " · " + record.url)
           << "</span></span><span></span></a>";
    }
    body << "</div>";
  }
  return PageChrome("History", body.str());
}

std::string SettingsHtml() {
  const std::string appearance = Setting("appearance", "system");
  const std::string searchEngine = Setting("searchEngine", "google");
  const std::string customSearchUrl = Setting("customSearchUrl", "https://www.google.com/search?q={query}");
  const std::string newTabPage = Setting("newTabPage", "blank");
  const std::string sidebarVisible = Setting("sidebarVisible", "show") == "hide" ? "hide" : "show";

  auto chip = [](const std::string& key, const std::string& current, const std::string& value, const std::string& label) {
    return "<a class=\"chip" + std::string(current == value ? " selected" : "") + "\" href=\"" +
           ActionLink(key, value, "fubuki://settings/") + "\">" + HtmlEscape(label) + "</a>";
  };

  std::ostringstream body;
  body << "<section class=\"section\">"
       << "<div class=\"field\"><span>Appearance</span><div class=\"segmented\">"
       << chip("appearance", appearance, "system", "System")
       << chip("appearance", appearance, "light", "Light")
       << chip("appearance", appearance, "dark", "Dark")
       << "</div></div>"
       << "<div class=\"field\"><span>Search engine</span><div class=\"segmented\">"
       << chip("searchEngine", searchEngine, "google", "Google")
       << chip("searchEngine", searchEngine, "duckduckgo", "DuckDuckGo")
       << chip("searchEngine", searchEngine, "bing", "Bing")
       << chip("searchEngine", searchEngine, "custom", "Custom")
       << "</div><form class=\"inline-form\" action=\"fubuki://settings/set\" method=\"get\"><input type=\"hidden\" name=\"key\" value=\"customSearchUrl\"><input type=\"hidden\" name=\"return\" value=\"fubuki://settings/\"><input name=\"value\" value=\""
       << HtmlEscape(customSearchUrl) << "\" placeholder=\"https://example.com/search?q={query}\"><button class=\"button\">Save</button></form></div>"
       << "<div class=\"field\"><span>New tab page</span><div class=\"segmented\">"
       << chip("newTabPage", newTabPage, "blank", "Blank")
       << chip("newTabPage", newTabPage, "home", "Home")
       << "</div></div>"
       << "<div class=\"field\"><span>Clear browsing data</span><div class=\"segmented\">"
       << "<a class=\"chip danger\" href=\"" << ActionLink("clearData", "history", "fubuki://settings/") << "\">History</a>"
       << "<a class=\"chip danger\" href=\"" << ActionLink("clearData", "bookmarks", "fubuki://settings/") << "\">Bookmarks</a>"
       << "<a class=\"chip danger\" href=\"" << ActionLink("clearData", "downloads", "fubuki://settings/") << "\">Downloads</a>"
       << "<a class=\"chip danger\" href=\"" << ActionLink("clearData", "all", "fubuki://settings/") << "\">All</a>"
       << "</div></div>"
       << "<div class=\"field\"><span>Sidebar visibility</span><div class=\"segmented\">"
       << chip("sidebarVisible", sidebarVisible, "show", "Show")
       << chip("sidebarVisible", sidebarVisible, "hide", "Hide")
       << "</div></div></section>";
  return PageChrome("Settings", body.str());
}

std::string NewTabHtml() {
  std::ostringstream html;
  html << R"(<!doctype html><html><head><meta charset="utf-8"><title>New Tab</title>)" << FubukiFaviconLink() << R"(<style>
*{box-sizing:border-box}html,body{height:100%}body{margin:0;display:grid;place-items:center;background:#fff;color:#17191d;font:15px -apple-system,BlinkMacSystemFont,"SF Pro Text","Helvetica Neue",sans-serif}main{width:min(620px,calc(100vw - 40px));display:grid;gap:22px;justify-items:center}.logo{width:58px;height:58px}h1{margin:0;font-size:32px;line-height:1;font-weight:720}form{width:100%}input{width:100%;height:42px;border:1px solid rgb(24 32 44/.16);border-radius:8px;padding:0 12px;font:inherit;outline:0}input:focus{border-color:#2563eb;box-shadow:0 0 0 3px rgb(37 99 235/.18)}
</style></head><body><main>)"
       << FubukiLogoSvg() << R"(<h1>Fubuki Browser Alpha</h1><form action="fubuki://newtab/search" method="get"><input name="q" autofocus autocomplete="off" placeholder="Search or enter URL"></form></main></body></html>)";
  return html.str();
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
  headers.insert({"Content-Type", mimeType_ + (mimeType_.rfind("text/", 0) == 0 ? "; charset=utf-8" : "")});
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
  if (url.rfind("fubuki://bookmarks/", 0) == 0) {
    LoadText(BookmarksHtml(), "text/html", 200);
    return true;
  }
  if (url.rfind("fubuki://downloads/", 0) == 0) {
    LoadText(DownloadsHtml(), "text/html", 200);
    return true;
  }
  if (url.rfind("fubuki://history/", 0) == 0) {
    LoadText(HistoryHtml(), "text/html", 200);
    return true;
  }
  if (url.rfind("fubuki://settings", 0) == 0) {
    LoadText(SettingsHtml(), "text/html", 200);
    return true;
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
