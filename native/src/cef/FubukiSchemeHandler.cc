#include "cef/FubukiSchemeHandler.h"

#include <algorithm>
#include <cstdlib>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <sstream>
#include <unordered_map>
#include <vector>

#include <sqlite3.h>

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
};

std::filesystem::path ProfilePath() {
  const char *home = std::getenv("HOME");
  return home ? std::filesystem::path(home) /
                    "Library/Application Support/Fubuki Browser Alpha"
              : std::filesystem::temp_directory_path() / "Fubuki Browser Alpha";
}

std::filesystem::path DatabasePath() {
  return ProfilePath() / "fubuki.sqlite3";
}

std::string MimeForPath(const std::string &path) {
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

std::string HtmlEscape(const std::string &value) {
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

void Execute(sqlite3 *db, const std::string &sql) {
  sqlite3_exec(db, sql.c_str(), nullptr, nullptr, nullptr);
}

std::string ColumnText(sqlite3_stmt *statement, int column) {
  const unsigned char *text = sqlite3_column_text(statement, column);
  return text ? reinterpret_cast<const char *>(text) : "";
}

sqlite3 *OpenDatabase() {
  std::filesystem::create_directories(ProfilePath());
  sqlite3 *db = nullptr;
  if (sqlite3_open(DatabasePath().string().c_str(), &db) != SQLITE_OK) {
    return nullptr;
  }
  Execute(db, "CREATE TABLE IF NOT EXISTS settings(key TEXT PRIMARY KEY, value "
              "TEXT NOT NULL)");
  Execute(db, "CREATE TABLE IF NOT EXISTS bookmarks(id INTEGER PRIMARY KEY "
              "AUTOINCREMENT,title TEXT NOT NULL,url TEXT NOT NULL "
              "UNIQUE,favicon_url TEXT,created_at TEXT NOT NULL)");
  Execute(db, "CREATE TABLE IF NOT EXISTS history(id INTEGER PRIMARY KEY "
              "AUTOINCREMENT,title TEXT NOT NULL,url TEXT NOT NULL,created_at "
              "TEXT NOT NULL)");
  Execute(db, "CREATE TABLE IF NOT EXISTS downloads(id INTEGER PRIMARY KEY "
              "AUTOINCREMENT,url TEXT,path TEXT,state TEXT,percent INTEGER "
              "DEFAULT 0,created_at TEXT NOT NULL,updated_at TEXT)");
  Execute(db,
          "CREATE TABLE IF NOT EXISTS logs(id INTEGER PRIMARY KEY "
          "AUTOINCREMENT,level TEXT,message TEXT,created_at TEXT NOT NULL)");
  Execute(db, "CREATE TABLE IF NOT EXISTS site_permissions(origin TEXT NOT "
              "NULL,permission TEXT NOT NULL,value TEXT NOT NULL,updated_at "
              "TEXT NOT NULL,PRIMARY KEY(origin,permission))");
  return db;
}

std::string Setting(const std::string &key, const std::string &fallback = "") {
  sqlite3 *db = OpenDatabase();
  if (!db)
    return fallback;
  sqlite3_stmt *statement = nullptr;
  sqlite3_prepare_v2(db, "SELECT value FROM settings WHERE key=?", -1,
                     &statement, nullptr);
  sqlite3_bind_text(statement, 1, key.c_str(), static_cast<int>(key.size()),
                    SQLITE_TRANSIENT);
  std::string value = fallback;
  if (sqlite3_step(statement) == SQLITE_ROW) {
    value = ColumnText(statement, 0);
  }
  sqlite3_finalize(statement);
  sqlite3_close(db);
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
  const char *lang = std::getenv("LANG");
  return lang && std::string(lang).rfind("ja", 0) == 0 ? "ja" : "en";
}

static const std::unordered_map<std::string, std::string> kJaLabels = {
    {"Bookmarks", "ブックマーク"},
    {"Downloads", "ダウンロード"},
    {"History", "履歴"},
    {"Settings", "設定"},
    {"Debug", "デバッグ"},
    {"New Tab", "新しいタブ"},
    {"Search or enter URL", "検索語句またはURLを入力"},
    {"No bookmarks", "ブックマークはまだありません"},
    {"No downloads", "ダウンロードはまだありません"},
    {"No history", "履歴はまだありません"},
    {"Delete", "削除"},
    {"Remove", "削除"},
    {"Open", "開く"},
    {"Reveal", "Finderで表示"},
    {"Clear downloads", "ダウンロード履歴を消去"},
    {"Search history", "履歴を検索"},
    {"Clear last hour", "直近1時間を消去"},
    {"Clear today", "今日の履歴を消去"},
    {"Clear all", "すべて消去"},
    {"Earlier", "以前"},
    {"General", "一般"},
    {"Appearance", "外観"},
    {"Language", "言語"},
    {"Tabs", "タブ"},
    {"Windows", "ウィンドウ"},
    {"Search", "検索"},
    {"Privacy", "プライバシー"},
    {"Developer", "開発"},
    {"Downloads section", "ダウンロード"},
    {"Search settings", "設定を検索"},
    {"System", "システム設定"},
    {"Light", "ライト"},
    {"Dark", "ダーク"},
    {"English", "英語"},
    {"Japanese", "日本語"},
    {"New tab", "新しいタブ"},
    {"Restore previous session", "前回のセッションを復元"},
    {"Home page", "ホームページ"},
    {"Home page URL", "ホームページURL"},
    {"Save", "保存"},
    {"Reset", "リセット"},
    {"Show sidebar", "サイドバーを表示"},
    {"Hide sidebar", "サイドバーを隠す"},
    {"Sidebar width", "サイドバー幅"},
    {"Reset sidebar width", "サイドバー幅をリセット"},
    {"Blank new tab", "空の新規タブ"},
    {"Home on new tab", "新規タブでホームを開く"},
    {"Default zoom level", "既定の表示倍率"},
    {"Ask before download", "保存前に確認"},
    {"Download automatically", "自動でダウンロード"},
    {"Download directory", "ダウンロード先"},
    {"Open DevTools", "DevToolsを開く"},
    {"Debug page", "デバッグページ"},
    {"Profile path", "プロファイルパス"},
    {"Windows and tabs", "ウィンドウとタブ"},
    {"Registered commands", "登録済みコマンド"},
    {"Recent events", "最近のイベント"},
    {"Logs", "ログ"},
    {"Actions", "操作"},
};

std::string Label(const std::string &key) {
  const bool ja = BrowserLanguage() == "ja";
  if (!ja)
    return key;
  const auto it = kJaLabels.find(key);
  return it != kJaLabels.end() ? it->second : key;
}

std::vector<Record> QueryRecords(const std::string &table, int limit) {
  sqlite3 *db = OpenDatabase();
  if (!db)
    return {};

  const std::string sql =
      table == "bookmarks" ? "SELECT title,url,favicon_url,'','',0,created_at "
                             "FROM bookmarks ORDER BY id DESC LIMIT ?"
      : table == "history" ? "SELECT title,url,'','','',0,created_at FROM "
                             "history ORDER BY id DESC LIMIT ?"
      : table == "logs"
          ? "SELECT message,'','',level,'',0,created_at FROM logs ORDER BY id "
            "DESC LIMIT ?"
          : "SELECT "
            "'',url,'',path,state,percent,COALESCE(updated_at,created_at) FROM "
            "downloads ORDER BY COALESCE(updated_at,created_at) DESC,id DESC "
            "LIMIT ?";
  sqlite3_stmt *statement = nullptr;
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

std::string FubukiLogoSvg(const std::string &className = "logo") {
  return "<svg class=\"" + className +
         "\" width=\"512\" height=\"512\" viewBox=\"0 0 512 512\" "
         "fill=\"none\" xmlns=\"http://www.w3.org/2000/svg\">"
         "<path d=\"M128 440L183.252 248.366M470 72L252.28 72C238.617 72 "
         "226.68 81.2317 223.244 94.4554L183.252 248.366M183.252 "
         "248.366H363.904\" stroke=\"url(#paint0_linear_7_2)\" "
         "stroke-width=\"25\" stroke-linecap=\"round\"/>"
         "<path d=\"M95.6021 142.602L148.204 195.204M148.204 195.204L43.0001 "
         "195.204M148.204 195.204L95.6021 247.806M148.204 "
         "195.204V300.408M148.204 195.204L200.806 247.806M148.204 "
         "195.204V90M148.204 195.204L200.806 142.602M148.204 195.204H253.408\" "
         "stroke=\"#1AADEB\" stroke-width=\"5\" stroke-linecap=\"round\"/>"
         "<defs><linearGradient id=\"paint0_linear_7_2\" x1=\"257.282\" "
         "y1=\"72\" x2=\"257.282\" y2=\"476.326\" "
         "gradientUnits=\"userSpaceOnUse\"><stop stop-color=\"#FF9686\"/><stop "
         "offset=\"1\" stop-color=\"#A7ABE0\"/></linearGradient></defs>"
         "</svg>";
}

std::string FubukiFaviconLink() {
  return "<link rel=\"icon\" type=\"image/svg+xml\" "
         "href=\"data:image/svg+xml," +
         CefURIEncode(FubukiLogoSvg(), false).ToString() + "\">";
}

std::string PageChrome(const std::string &title, const std::string &body) {
  const std::string appearance = BrowserAppearance();
  const std::string lang = BrowserLanguage();
  std::ostringstream html;
  html << "<!doctype html><html lang=\"" << HtmlEscape(lang)
       << "\" data-appearance=\"" << HtmlEscape(appearance)
       << "\"><head><meta charset=\"utf-8\"><title>" << HtmlEscape(Label(title))
       << "</title>" << FubukiFaviconLink() << R"(<style>
*{box-sizing:border-box}html{scroll-behavior:smooth}body{margin:0;background:var(--bg);color:var(--text);font:14px -apple-system,BlinkMacSystemFont,"SF Pro Text","Hiragino Sans","Hiragino Kaku Gothic ProN","Yu Gothic","Helvetica Neue",sans-serif;letter-spacing:0;--bg:#f5f6f8;--surface:#fff;--surface-2:#eef1f4;--text:#15171a;--muted:#66707c;--line:rgb(22 28 36/.12);--hover:rgb(22 28 36/.055);--active:rgb(28 101 242/.1);--accent:#1f6feb;--danger:#b42318;--shadow:0 1px 2px rgb(18 24 32/.06)}
html[data-appearance=dark] body{--bg:#14161a;--surface:#1d2025;--surface-2:#252932;--text:#f4f6f8;--muted:#a7b0bd;--line:rgb(255 255 255/.12);--hover:rgb(255 255 255/.07);--active:rgb(111 168 255/.14);--accent:#76a9ff;--danger:#ff8a80;--shadow:none;color-scheme:dark}
@media(prefers-color-scheme:dark){html[data-appearance=system] body{--bg:#14161a;--surface:#1d2025;--surface-2:#252932;--text:#f4f6f8;--muted:#a7b0bd;--line:rgb(255 255 255/.12);--hover:rgb(255 255 255/.07);--active:rgb(111 168 255/.14);--accent:#76a9ff;--danger:#ff8a80;--shadow:none;color-scheme:dark}}
@keyframes pageIn{from{opacity:0;transform:translateY(10px)}to{opacity:1;transform:translateY(0)}}@keyframes rowIn{from{opacity:0;transform:translateY(6px)}to{opacity:1;transform:translateY(0)}}@keyframes focusPulse{0%{box-shadow:0 0 0 0 color-mix(in srgb,var(--accent) 25%,transparent)}100%{box-shadow:0 0 0 6px transparent}}
main{width:min(1040px,calc(100vw - 48px));margin:0 auto;padding:34px 0 56px;animation:pageIn .32s cubic-bezier(.2,.8,.2,1)}header{display:flex;align-items:center;gap:12px;margin-bottom:24px}.logo{width:34px;height:34px}h1{font-size:30px;line-height:1.08;margin:0;font-weight:720}h2{font-size:13px;margin:14px 0 5px;color:var(--muted);font-weight:680}a{color:inherit}.list{display:grid;gap:7px}.row{min-height:48px;display:grid;grid-template-columns:22px minmax(0,1fr) auto;align-items:center;gap:10px;padding:9px 10px;border:1px solid var(--line);border-radius:7px;background:var(--surface);box-shadow:var(--shadow);text-decoration:none;animation:rowIn .28s cubic-bezier(.2,.8,.2,1);transition:background .16s ease,border-color .16s ease,transform .16s ease}.row:hover{background:var(--hover);border-color:color-mix(in srgb,var(--line) 55%,var(--accent));transform:translateY(-1px)}.row>a{min-width:0;text-decoration:none}.favicon{width:16px;height:16px;border-radius:4px;background:linear-gradient(135deg,#25a8d7,#6d7edc 58%,#f08072)}.favicon img{width:16px;height:16px;border-radius:4px}.title{min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;font-weight:640}.meta{min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;color:var(--muted);font-size:12px;line-height:1.45}.button,.chip{min-height:30px;display:inline-grid;place-items:center;border:1px solid var(--line);border-radius:7px;padding:0 10px;background:var(--surface);color:var(--text);text-decoration:none;font:inherit;font-weight:620;transition:background .16s ease,border-color .16s ease,color .16s ease,transform .16s ease}.button:hover,.chip:hover{background:var(--hover);transform:translateY(-1px)}.danger{color:var(--danger)}.empty{color:var(--muted);padding:18px 0}.section{display:grid;gap:14px}.field{display:grid;gap:11px;padding:14px;border:1px solid var(--line);border-radius:7px;background:var(--surface);box-shadow:var(--shadow);animation:rowIn .28s cubic-bezier(.2,.8,.2,1);scroll-margin-top:18px}.field>span{font-weight:680}.segmented{display:flex;flex-wrap:wrap;gap:8px}.selected{border-color:color-mix(in srgb,var(--accent) 70%,var(--line));background:var(--active);color:var(--accent)}input{height:34px;min-width:220px;border:1px solid var(--line);border-radius:7px;padding:0 10px;background:var(--surface);color:var(--text);font:inherit;outline:0;transition:border-color .16s ease,box-shadow .16s ease,background .16s ease}input:focus{border-color:var(--accent);animation:focusPulse .5s ease}.inline-form{display:flex;gap:8px;align-items:center;flex-wrap:wrap}.settings-layout{display:grid;grid-template-columns:220px minmax(0,1fr);gap:18px;align-items:start}.settings-nav{position:sticky;top:20px;display:grid;gap:4px;padding:8px;border:1px solid var(--line);border-radius:7px;background:var(--surface);box-shadow:var(--shadow)}.settings-nav a{min-height:34px;display:flex;align-items:center;padding:0 10px;border-radius:6px;color:var(--muted);font-weight:640;text-decoration:none;transition:background .16s ease,color .16s ease,transform .16s ease}.settings-nav a:hover{background:var(--hover);color:var(--text);transform:translateX(2px)}.settings-content{display:grid;gap:14px}.settings-search{margin-bottom:0}.section-kicker{color:var(--muted);font-size:12px}.switch-row{display:grid;grid-template-columns:minmax(0,1fr) auto;gap:12px;align-items:center}@media(max-width:760px){main{width:min(100% - 28px,1040px);padding-top:24px}.settings-layout{grid-template-columns:1fr}.settings-nav{position:static;grid-template-columns:repeat(2,minmax(0,1fr))}input{min-width:0;width:100%}.row{grid-template-columns:20px minmax(0,1fr)}}@media(prefers-reduced-motion:reduce){*,*::before,*::after{animation:none!important;scroll-behavior:auto!important;transition:none!important}}
html[data-appearance=dark] body{--bg:#14161a;--surface:#1d2025;--surface-2:#252932;--text:#f4f6f8;--muted:#a7b0bd;--line:rgb(255 255 255/.12);--hover:rgb(255 255 255/.07);--active:rgb(111 168 255/.14);--accent:#76a9ff;--danger:#ff8a80;--shadow:none;color-scheme:dark}
</style></head><body><main><header>)"
       << FubukiLogoSvg() << "<h1>" << HtmlEscape(Label(title))
       << "</h1></header>" << body << "</main></body></html>";
  return html.str();
}

std::string ActionLink(const std::string &key, const std::string &value,
                       const std::string &returnUrl) {
  return "fubuki://settings/set?key=" + key +
         "&value=" + CefURIEncode(value, false).ToString() +
         "&return=" + CefURIEncode(returnUrl, false).ToString();
}

std::string FileName(const std::string &path, const std::string &url) {
  const std::string source = path.empty() ? url : path;
  const size_t slash = source.find_last_of("/\\");
  return slash == std::string::npos ? source : source.substr(slash + 1);
}

std::string BookmarksHtml() {
  std::ostringstream body;
  const auto records = QueryRecords("bookmarks", 500);
  if (records.empty()) {
    body << "<p class=\"empty\">" << Label("No bookmarks") << "</p>";
  } else {
    body << "<div class=\"list\">";
    for (const auto &record : records) {
      body << "<div class=\"row\"><span class=\"favicon\">";
      if (!record.faviconUrl.empty()) {
        body << "<img alt=\"\" src=\"" << HtmlEscape(record.faviconUrl)
             << "\">";
      }
      body << "</span><a href=\"" << HtmlEscape(record.url) << "\" title=\""
           << HtmlEscape(record.url) << "\"><div class=\"title\">"
           << HtmlEscape(record.title.empty() ? record.url : record.title)
           << "</div><div class=\"meta\">" << HtmlEscape(record.url)
           << "</div></a>"
           << "<a class=\"button danger\" href=\""
           << ActionLink("removeBookmark", record.url, "fubuki://bookmarks/")
           << "\">" << Label("Delete") << "</a></div>";
    }
    body << "</div>";
  }
  return PageChrome("Bookmarks", body.str());
}

std::string DownloadsHtml() {
  std::ostringstream body;
  const auto records = QueryRecords("downloads", 50);
  body << "<div class=\"segmented\" style=\"margin-bottom:12px\"><a "
          "class=\"chip danger\" href=\""
       << ActionLink("clearData", "downloads", "fubuki://downloads/") << "\">"
       << Label("Clear downloads") << "</a></div>";
  if (records.empty()) {
    body << "<p class=\"empty\">" << Label("No downloads") << "</p>";
  } else {
    body << "<div class=\"list\">";
    for (const auto &record : records) {
      body << "<article class=\"row\"><span "
              "aria-hidden=\"true\">↓</span><div><div class=\"title\">"
           << HtmlEscape(FileName(record.path, record.url))
           << "</div><div class=\"meta\">"
           << HtmlEscape(record.path.empty() ? record.url : record.path)
           << "</div></div><span class=\"segmented\">"
           << "<a class=\"chip\" href=\""
           << ActionLink("openDownload", record.path, "fubuki://downloads/")
           << "\">" << Label("Open") << "</a>"
           << "<a class=\"chip\" href=\""
           << ActionLink("revealDownload", record.path, "fubuki://downloads/")
           << "\">" << Label("Reveal") << "</a>"
           << "<a class=\"chip danger\" href=\""
           << ActionLink("removeDownload", record.path, "fubuki://downloads/")
           << "\">" << Label("Remove") << "</a>"
           << "</span></article><div class=\"meta\" style=\"padding:0 10px 6px "
              "42px\">"
           << HtmlEscape(record.state.empty() ? "unknown" : record.state) << " "
           << record.percent << "%</div>";
    }
    body << "</div>";
  }
  return PageChrome("Downloads", body.str());
}

std::string HistoryHtml() {
  std::ostringstream body;
  const auto records = QueryRecords("history", 500);
  body << "<form class=\"inline-form\" style=\"margin-bottom:12px\"><input "
          "type=\"search\" id=\"historySearch\" placeholder=\""
       << Label("Search history")
       << "\" oninput=\"for(const row of "
          "document.querySelectorAll('[data-history-row]')) "
          "row.style.display=row.textContent.toLowerCase().includes(this.value."
          "toLowerCase())?'grid':'none'\"></form>";
  body << "<div class=\"segmented\" style=\"margin-bottom:12px\"><a "
          "class=\"chip danger\" href=\""
       << ActionLink("clearHistoryRange", "lastHour", "fubuki://history/")
       << "\">" << Label("Clear last hour")
       << "</a><a class=\"chip danger\" href=\""
       << ActionLink("clearHistoryRange", "today", "fubuki://history/") << "\">"
       << Label("Clear today") << "</a><a class=\"chip danger\" href=\""
       << ActionLink("clearHistoryRange", "all", "fubuki://history/") << "\">"
       << Label("Clear all") << "</a></div>";
  if (records.empty()) {
    body << "<p class=\"empty\">" << Label("No history") << "</p>";
  } else {
    body << "<div class=\"list\">";
    std::string currentDate;
    for (const auto &record : records) {
      const std::string day = record.createdAt.size() >= 10
                                  ? record.createdAt.substr(0, 10)
                                  : Label("Earlier");
      if (day != currentDate) {
        currentDate = day;
        body << "<h2 style=\"font-size:13px;margin:12px 0 "
                "4px;color:var(--muted)\">"
             << HtmlEscape(day) << "</h2>";
      }
      body << "<div data-history-row class=\"row\"><span "
              "class=\"favicon\"></span><a href=\""
           << HtmlEscape(record.url) << "\" title=\"" << HtmlEscape(record.url)
           << "\"><span class=\"title\">"
           << HtmlEscape(record.title.empty() ? record.url : record.title)
           << "</span><span class=\"meta\">"
           << HtmlEscape(record.createdAt + " · " + record.url)
           << "</span></a><a class=\"button danger\" href=\""
           << ActionLink("removeHistory", record.url, "fubuki://history/")
           << "\">" << Label("Delete") << "</a></div>";
    }
    body << "</div>";
  }
  return PageChrome("History", body.str());
}

std::string SettingsHtml() {
  const std::string appearance = Setting("appearance", "system");
  const std::string searchEngine = Setting("searchEngine", "google");
  const std::string customSearchUrl =
      Setting("customSearchUrl", "https://www.google.com/search?q={query}");
  const std::string newTabPage = Setting("newTabPage", "blank");
  const std::string startupBehavior = Setting("startupBehavior", "newTab");
  const std::string askBeforeDownload = Setting("askBeforeDownload", "off");
  const std::string sidebarVisible =
      Setting("sidebarVisible", "show") == "hide" ? "hide" : "show";
  const std::string language = Setting("language", "system");

  auto chip = [](const std::string &key, const std::string &current,
                 const std::string &value, const std::string &label) {
    return "<a class=\"chip" +
           std::string(current == value ? " selected" : "") + "\" href=\"" +
           ActionLink(key, value, "fubuki://settings/") + "\">" +
           HtmlEscape(label) + "</a>";
  };

  std::ostringstream body;
  body
      << "<div class=\"settings-layout\">"
      << "<nav class=\"settings-nav\" aria-label=\"Settings sections\">"
      << "<a href=\"#general\">" << Label("General")
      << "</a><a href=\"#appearance\">" << Label("Appearance")
      << "</a><a href=\"#language\">" << Label("Language")
      << "</a><a href=\"#tabs\">" << Label("Tabs") << "</a>"
      << "<a href=\"#windows\">" << Label("Windows")
      << "</a><a href=\"#search\">" << Label("Search")
      << "</a><a href=\"#privacy\">" << Label("Privacy") << "</a>"
      << "<a href=\"#downloads\">" << Label("Downloads section")
      << "</a><a href=\"#developer\">" << Label("Developer") << "</a>"
      << "</nav><section class=\"settings-content\">"
      << "<form class=\"inline-form settings-search\"><input type=\"search\" "
         "placeholder=\""
      << Label("Search settings")
      << "\" oninput=\"for(const field of "
         "document.querySelectorAll('[data-setting-section]')) "
         "field.style.display=field.textContent.toLowerCase().includes(this."
         "value.toLowerCase())?'grid':'none'\"></form>"
      << "<div id=\"general\" class=\"field\" data-setting-section><span>"
      << Label("General")
      << "</span><div class=\"section-kicker\">Choose how Fubuki starts and "
         "where Home opens.</div><div class=\"segmented\">"
      << chip("startupBehavior", startupBehavior, "newTab", Label("New tab"))
      << chip("startupBehavior", startupBehavior, "restore",
              Label("Restore previous session"))
      << chip("startupBehavior", startupBehavior, "homePage",
              Label("Home page"))
      << "</div><form class=\"inline-form\" action=\"fubuki://settings/set\" "
         "method=\"get\"><input type=\"hidden\" name=\"key\" "
         "value=\"homeUrl\"><input type=\"hidden\" name=\"return\" "
         "value=\"fubuki://settings/\"><input name=\"value\" value=\""
      << HtmlEscape(Setting("homeUrl", "https://example.com"))
      << "\" placeholder=\"" << Label("Home page URL")
      << "\"><button class=\"button\">" << Label("Save")
      << "</button><a class=\"button\" href=\""
      << ActionLink("resetSetting", "startupBehavior", "fubuki://settings/")
      << "\">" << Label("Reset") << "</a></form></div>"
      << "<div id=\"appearance\" class=\"field\" data-setting-section><span>"
      << Label("Appearance")
      << "</span><div class=\"section-kicker\">Use a flat system, light, or "
         "dark internal page theme.</div><div class=\"segmented\">"
      << chip("appearance", appearance, "system", Label("System"))
      << chip("appearance", appearance, "light", Label("Light"))
      << chip("appearance", appearance, "dark", Label("Dark"))
      << "</div><a class=\"button\" href=\""
      << ActionLink("resetSetting", "appearance", "fubuki://settings/") << "\">"
      << Label("Reset") << "</a></div>"
      << "<div id=\"language\" class=\"field\" data-setting-section><span>"
      << Label("Language")
      << "</span><div class=\"section-kicker\">Choose UI language. System "
         "follows your macOS language when possible.</div><div "
         "class=\"segmented\">"
      << chip("language", language, "system", Label("System"))
      << chip("language", language, "ja", Label("Japanese"))
      << chip("language", language, "en", Label("English"))
      << "</div><a class=\"button\" href=\""
      << ActionLink("resetSetting", "language", "fubuki://settings/") << "\">"
      << Label("Reset") << "</a></div>"
      << "<div id=\"tabs\" class=\"field\" data-setting-section><span>"
      << Label("Tabs")
      << "</span><div class=\"section-kicker\">Tune the new tab destination "
         "and page zoom default.</div><div class=\"segmented\">"
      << chip("newTabPage", newTabPage, "blank", Label("Blank new tab"))
      << chip("newTabPage", newTabPage, "home", Label("Home on new tab"))
      << "</div><form class=\"inline-form\" action=\"fubuki://settings/set\" "
         "method=\"get\"><input type=\"hidden\" name=\"key\" "
         "value=\"defaultZoomLevel\"><input type=\"hidden\" name=\"return\" "
         "value=\"fubuki://settings/\"><input name=\"value\" value=\""
      << HtmlEscape(Setting("defaultZoomLevel", "0")) << "\" placeholder=\""
      << Label("Default zoom level") << "\"><button class=\"button\">"
      << Label("Save") << "</button><a class=\"button\" href=\""
      << ActionLink("resetSetting", "defaultZoomLevel", "fubuki://settings/")
      << "\">" << Label("Reset") << "</a></form></div>"
      << "<div id=\"windows\" class=\"field\" data-setting-section><span>"
      << Label("Windows")
      << "</span><div class=\"section-kicker\">Control browser chrome "
         "visibility.</div><div class=\"segmented\">"
      << chip("sidebarVisible", sidebarVisible, "show", Label("Show sidebar"))
      << chip("sidebarVisible", sidebarVisible, "hide", Label("Hide sidebar"))
      << "</div><form class=\"inline-form\" action=\"fubuki://settings/set\" "
         "method=\"get\"><input type=\"hidden\" name=\"key\" "
         "value=\"sidebarWidth\"><input type=\"hidden\" name=\"return\" "
         "value=\"fubuki://settings/\"><input name=\"value\" value=\""
      << HtmlEscape(Setting("sidebarWidth", "196")) << "\" placeholder=\""
      << Label("Sidebar width") << "\"><button class=\"button\">"
      << Label("Save") << "</button><a class=\"button\" href=\""
      << ActionLink("resetSetting", "sidebarWidth", "fubuki://settings/")
      << "\">" << Label("Reset sidebar width") << "</a></form></div>"
      << "<div id=\"search\" class=\"field\" data-setting-section><span>"
      << Label("Search")
      << "</span><div class=\"section-kicker\">Set the engine used from the "
         "omnibox and new tab page.</div><div class=\"segmented\">"
      << chip("searchEngine", searchEngine, "google", "Google")
      << chip("searchEngine", searchEngine, "duckduckgo", "DuckDuckGo")
      << chip("searchEngine", searchEngine, "bing", "Bing")
      << chip("searchEngine", searchEngine, "custom", "Custom")
      << "</div><form class=\"inline-form\" action=\"fubuki://settings/set\" "
         "method=\"get\"><input type=\"hidden\" name=\"key\" "
         "value=\"customSearchUrl\"><input type=\"hidden\" name=\"return\" "
         "value=\"fubuki://settings/\"><input name=\"value\" value=\""
      << HtmlEscape(customSearchUrl)
      << "\" placeholder=\"https://example.com/search?q={query}\"><button "
         "class=\"button\">"
      << Label("Save") << "</button><a class=\"button\" href=\""
      << ActionLink("resetSetting", "searchEngine", "fubuki://settings/")
      << "\">" << Label("Reset") << "</a></form></div>"
      << "<div id=\"privacy\" class=\"field\" data-setting-section><span>"
      << Label("Privacy")
      << "</span><div class=\"section-kicker\">Clear local browsing records "
         "and web storage.</div><div class=\"segmented\">"
      << "<a class=\"chip danger\" href=\""
      << ActionLink("clearData", "history", "fubuki://settings/") << "\">"
      << Label("History") << "</a>"
      << "<a class=\"chip danger\" href=\""
      << ActionLink("clearData", "cookies", "fubuki://settings/")
      << "\">Cookies</a>"
      << "<a class=\"chip danger\" href=\""
      << ActionLink("clearData", "cache", "fubuki://settings/")
      << "\">Cache</a>"
      << "<a class=\"chip danger\" href=\""
      << ActionLink("clearData", "downloads", "fubuki://settings/") << "\">"
      << Label("Downloads") << "</a>"
      << "<a class=\"chip danger\" href=\""
      << ActionLink("clearData", "all", "fubuki://settings/") << "\">"
      << Label("Clear all") << "</a>"
      << "</div></div>"
      << "<div id=\"downloads\" class=\"field\" data-setting-section><span>"
      << Label("Downloads section")
      << "</span><div class=\"section-kicker\">Set download confirmation and "
         "the default folder.</div><div class=\"segmented\">"
      << chip("askBeforeDownload", askBeforeDownload, "on",
              Label("Ask before download"))
      << chip("askBeforeDownload", askBeforeDownload, "off",
              Label("Download automatically"))
      << "</div><form class=\"inline-form\" action=\"fubuki://settings/set\" "
         "method=\"get\"><input type=\"hidden\" name=\"key\" "
         "value=\"downloadDirectory\"><input type=\"hidden\" name=\"return\" "
         "value=\"fubuki://settings/\"><input name=\"value\" value=\""
      << HtmlEscape(Setting("downloadDirectory", "")) << "\" placeholder=\""
      << Label("Download directory") << "\"><button class=\"button\">"
      << Label("Save") << "</button><a class=\"button\" href=\""
      << ActionLink("resetSetting", "downloadDirectory", "fubuki://settings/")
      << "\">" << Label("Reset") << "</a></form></div>"
      << "<div class=\"field\" data-setting-section><span>Shortcuts</span><div "
         "class=\"meta\">Cmd+T, Cmd+N, Cmd+Shift+N, Cmd+W, Cmd+Shift+T, Cmd+L, "
         "Cmd+R, Cmd+F, Cmd+,, Cmd+Plus, Cmd+Minus, Cmd+0</div></div>"
      << "<div id=\"developer\" class=\"field\" data-setting-section><span>"
      << Label("Developer")
      << "</span><div class=\"section-kicker\">Inspect the app shell and "
         "internal diagnostics.</div><div class=\"segmented\"><a "
         "class=\"chip\" href=\""
      << ActionLink("openDevTools", "1", "fubuki://settings/") << "\">"
      << Label("Open DevTools")
      << "</a><a class=\"chip\" href=\"fubuki://debug/\">"
      << Label("Debug page") << "</a></div></div>"
      << "</section></div>";
  return PageChrome("Settings", body.str());
}

std::string DebugHtml() {
  std::ostringstream body;
  BrowserAppController *app = GetBrowserAppController();
  body << "<section class=\"section\">";
  body << "<div class=\"field\"><span>Bridge</span><div class=\"meta\">Version "
          "1</div></div>";
  body << "<div class=\"field\"><span>" << Label("Profile path")
       << "</span><div class=\"meta\">" << HtmlEscape(ProfilePath().string())
       << "</div></div>";
  if (app) {
    body << "<div class=\"field\"><span>" << Label("Windows and tabs")
         << "</span><div class=\"list\">";
    for (auto *window : app->Windows()) {
      body << "<article class=\"row\"><span>▣</span><div><div class=\"title\">"
           << HtmlEscape(window->WindowId())
           << (window->IsPrivate() ? " (Private)" : "")
           << "</div><div class=\"meta\">";
      const auto tabs = window->Tabs().GetTabs();
      for (const auto &tab : tabs) {
        body << HtmlEscape((tab.isActive ? "* " : "") +
                           (tab.title.empty() ? tab.url : tab.title))
             << " ";
      }
      body << "</div></div><span class=\"meta\">" << tabs.size()
           << " tabs</span></article>";
    }
    body << "</div></div>";

    body << "<div class=\"field\"><span>" << Label("Registered commands")
         << "</span><div class=\"list\">";
    if (auto *active = app->ActiveWindow()) {
      auto commands = active->Commands().List();
      for (size_t i = 0; i < commands->GetSize(); ++i) {
        auto command = commands->GetDictionary(i);
        if (!command)
          continue;
        body
            << "<article class=\"row\"><span>⌘</span><div><div class=\"title\">"
            << HtmlEscape(command->GetString("title"))
            << "</div><div class=\"meta\">"
            << HtmlEscape(command->GetString("id"))
            << "</div></div><span class=\"meta\">"
            << HtmlEscape(command->GetString("shortcut"))
            << "</span></article>";
      }
    }
    body << "</div></div>";

    body << "<div class=\"field\"><span>" << Label("Recent events")
         << "</span><div class=\"list\">";
    for (const auto &event : app->Events().RecentEvents()) {
      body << "<article class=\"row\"><span>•</span><div><div class=\"title\">"
           << HtmlEscape(event.name) << "</div><div class=\"meta\">"
           << HtmlEscape(event.windowId + " " + event.tabId + " " +
                         event.message)
           << "</div></div><span></span></article>";
    }
    body << "</div></div>";
  }

  body << "<div class=\"field\"><span>" << Label("Logs")
       << "</span><div class=\"list\">";
  const auto logs = QueryRecords("logs", 80);
  for (const auto &record : logs) {
    body << "<article class=\"row\"><span>i</span><div><div class=\"title\">"
         << HtmlEscape(record.title) << "</div><div class=\"meta\">"
         << HtmlEscape(record.createdAt + " " + record.path)
         << "</div></div><span></span></article>";
  }
  body << "</div></div>";
  body << "<div class=\"field\"><span>" << Label("Actions")
       << "</span><div class=\"segmented\"><a class=\"chip\" href=\""
       << ActionLink("openDevTools", "1", "fubuki://debug/") << "\">"
       << Label("Open DevTools") << "</a></div></div>";
  body << "</section>";
  return PageChrome("Debug", body.str());
}

std::string NewTabHtml() {
  const std::string appearance = BrowserAppearance();
  const std::string lang = BrowserLanguage();
  std::ostringstream html;
  html
      << "<!doctype html><html lang=\"" << HtmlEscape(lang)
      << "\" data-appearance=\"" << HtmlEscape(appearance)
      << R"("><head><meta charset="utf-8"><title>)" << Label("New Tab")
      << R"(</title>)" << FubukiFaviconLink() << R"(<style>
*{box-sizing:border-box}html,body{height:100%}@keyframes pageIn{from{opacity:0;transform:translateY(10px)}to{opacity:1;transform:translateY(0)}}@keyframes focusPulse{0%{box-shadow:0 0 0 0 rgb(31 111 235/.24)}100%{box-shadow:0 0 0 7px transparent}}body{margin:0;display:grid;place-items:center;background:#f5f6f8;color:#15171a;font:15px -apple-system,BlinkMacSystemFont,"SF Pro Text","Hiragino Sans","Hiragino Kaku Gothic ProN","Yu Gothic","Helvetica Neue",sans-serif;letter-spacing:0}html[data-appearance=dark] body{background:#14161a;color:#f4f6f8;color-scheme:dark}main{width:min(620px,calc(100vw - 40px));display:grid;gap:20px;justify-items:center;animation:pageIn .34s cubic-bezier(.2,.8,.2,1)}.logo{width:58px;height:58px}h1{margin:0;font-size:32px;line-height:1;font-weight:720}form{width:100%}input{width:100%;height:44px;border:1px solid rgb(24 32 44/.14);border-radius:7px;background:#fff;padding:0 13px;font:inherit;outline:0;transition:border-color .16s ease,background .16s ease}html[data-appearance=dark] input{background:#1d2025;border-color:rgb(255 255 255/.12);color:#f4f6f8}@media(prefers-color-scheme:dark){html[data-appearance=system] body{background:#14161a;color:#f4f6f8;color-scheme:dark}html[data-appearance=system] input{background:#1d2025;border-color:rgb(255 255 255/.12);color:#f4f6f8}}input:focus{border-color:#1f6feb;animation:focusPulse .5s ease}@media(prefers-reduced-motion:reduce){*,*::before,*::after{animation:none!important;transition:none!important}}html[data-appearance=dark] body{background:#14161a;color:#f4f6f8;color-scheme:dark}html[data-appearance=dark] input{background:#1d2025;border-color:rgb(255 255 255/.12);color:#f4f6f8}
</style></head><body><main>)"
      << FubukiLogoSvg()
      << R"(<h1>Fubuki Browser Alpha</h1><form action="fubuki://newtab/search" method="get"><input name="q" autofocus autocomplete="off" placeholder=")"
      << Label("Search or enter URL") << R"("></form></main></body></html>)";
  return html.str();
}

}  // namespace

// PageCache implementation

PageCache &PageCache::Instance() {
  static PageCache instance;
  return instance;
}

bool PageCache::Get(const std::string &url, std::string &html) {
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

void PageCache::Set(const std::string &url, std::string html,
                    std::chrono::seconds ttl) {
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
  cache_[url] = {{std::move(html), std::chrono::steady_clock::now() + ttl},
                  order_.begin()};
}

void PageCache::Invalidate(const std::string &prefix) {
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

bool FubukiSchemeHandler::Open(CefRefPtr<CefRequest> request,
                               bool &handle_request, CefRefPtr<CefCallback>) {
  handle_request = true;
  return LoadRequest(request->GetURL().ToString());
}

void FubukiSchemeHandler::GetResponseHeaders(CefRefPtr<CefResponse> response,
                                             int64_t &response_length,
                                             CefString &) {
  response->SetStatus(status_);
  response->SetMimeType(mimeType_);
  CefResponse::HeaderMap headers;
  headers.insert({"Content-Type", mimeType_ + (mimeType_.rfind("text/", 0) == 0
                                                   ? "; charset=utf-8"
                                                   : "")});
  headers.insert({"Cache-Control", "no-store, max-age=0"});
  response->SetHeaderMap(headers);
  response_length = static_cast<int64_t>(data_.size());
}

bool FubukiSchemeHandler::Read(void *data_out, int bytes_to_read,
                               int &bytes_read,
                               CefRefPtr<CefResourceReadCallback>) {
  const size_t remaining = data_.size() - offset_;
  const size_t count =
      std::min<size_t>(remaining, static_cast<size_t>(bytes_to_read));
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

bool FubukiSchemeHandler::LoadRequest(const std::string &url) {
  offset_ = 0;
  auto &cache = PageCache::Instance();

  if (url.rfind("fubuki://newtab/", 0) == 0) {
    std::string html;
    if (cache.Get(url, html)) {
      LoadText(std::move(html), "text/html", 200);
    } else {
      html = NewTabHtml();
      cache.Set(url, html);
      LoadText(std::move(html), "text/html", 200);
    }
    return true;
  }
  if (url.rfind("fubuki://bookmarks/", 0) == 0) {
    std::string html;
    if (cache.Get(url, html)) {
      LoadText(std::move(html), "text/html", 200);
    } else {
      html = BookmarksHtml();
      cache.Set(url, html);
      LoadText(std::move(html), "text/html", 200);
    }
    return true;
  }
  if (url.rfind("fubuki://downloads/", 0) == 0) {
    std::string html;
    if (cache.Get(url, html)) {
      LoadText(std::move(html), "text/html", 200);
    } else {
      html = DownloadsHtml();
      cache.Set(url, html);
      LoadText(std::move(html), "text/html", 200);
    }
    return true;
  }
  if (url.rfind("fubuki://history/", 0) == 0) {
    std::string html;
    if (cache.Get(url, html)) {
      LoadText(std::move(html), "text/html", 200);
    } else {
      html = HistoryHtml();
      cache.Set(url, html);
      LoadText(std::move(html), "text/html", 200);
    }
    return true;
  }
  if (url.rfind("fubuki://settings", 0) == 0) {
    std::string html;
    if (cache.Get(url, html)) {
      LoadText(std::move(html), "text/html", 200);
    } else {
      html = SettingsHtml();
      cache.Set(url, html, std::chrono::seconds{2});
      LoadText(std::move(html), "text/html", 200);
    }
    return true;
  }
  if (url.rfind("fubuki://debug/", 0) == 0) {
    LoadText(DebugHtml(), "text/html", 200);
    return true;
  }
  if (url.rfind("fubuki://app/", 0) == 0) {
    const std::string path = ResolveAppPath(url);
    if (LoadFile(path, MimeForPath(path))) {
      return true;
    }
    LoadText("Fubuki UI build not found. Run `pnpm build` in ui/.",
             "text/plain", 404);
    return true;
  }
  LoadText("Not found", "text/plain", 404);
  return true;
}

bool FubukiSchemeHandler::LoadFile(const std::string &path,
                                   const std::string &mimeType) {
  std::ifstream file(path, std::ios::binary);
  if (!file) {
    return false;
  }
  std::ostringstream buffer;
  buffer << file.rdbuf();
  LoadText(buffer.str(), mimeType, 200);
  return true;
}

void FubukiSchemeHandler::LoadText(std::string body, std::string mimeType,
                                   int status) {
  data_ = std::move(body);
  mimeType_ = std::move(mimeType);
  status_ = status;
}

std::string FubukiSchemeHandler::ResolveAppPath(const std::string &url) const {
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

CefRefPtr<CefResourceHandler>
FubukiSchemeHandlerFactory::Create(CefRefPtr<CefBrowser>, CefRefPtr<CefFrame>,
                                   const CefString &, CefRefPtr<CefRequest>) {
  return new FubukiSchemeHandler(uiDistPath_);
}

}  // namespace fubuki
