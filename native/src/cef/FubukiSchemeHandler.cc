#include "cef/FubukiSchemeHandler.h"

#include <cstdlib>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <sstream>

#include <sqlite3.h>

#include "include/cef_parser.h"
#include "include/cef_version.h"
#include "utils/UrlUtils.h"

namespace fubuki {

namespace {

constexpr const char* kUnsplashSnow =
    "https://images.unsplash.com/photo-1483664852095-d6cc6870702d?auto=format&fit=crop&w=2400&q=85";

struct UiText {
  std::string general;
  std::string appearance;
  std::string search;
  std::string privacy;
  std::string about;
  std::string settings;
  std::string homepage;
  std::string startup;
  std::string downloadFolder;
  std::string theme;
  std::string language;
  std::string newTabBackground;
  std::string backgroundColor;
  std::string backgroundUrl;
  std::string save;
  std::string saved;
  std::string engine;
  std::string customSearchUrl;
  std::string permissions;
  std::string privacyBody;
  std::string version;
  std::string engineName;
  std::string profile;
  std::string bundle;
  std::string searchPlaceholder;
};

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

std::filesystem::path ProfilePath() {
  const char* home = std::getenv("HOME");
  return home ? std::filesystem::path(home) / "Library/Application Support/Fubuki Browser Alpha"
              : std::filesystem::temp_directory_path() / "Fubuki Browser Alpha";
}

std::filesystem::path DatabasePath() {
  return ProfilePath() / "fubuki.sqlite3";
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

std::string PathName(const std::string& url) {
  const std::string prefix = "fubuki://settings";
  if (url.rfind(prefix, 0) != 0) {
    return "general";
  }
  std::string path = url.substr(prefix.size());
  const size_t query = path.find_first_of("?#");
  if (query != std::string::npos) {
    path = path.substr(0, query);
  }
  if (path.empty() || path == "/") {
    return "general";
  }
  if (path[0] == '/') {
    path.erase(path.begin());
  }
  return path;
}

void Execute(sqlite3* db, const std::string& sql) {
  sqlite3_exec(db, sql.c_str(), nullptr, nullptr, nullptr);
}

std::string Setting(const std::string& key, const std::string& fallback = "") {
  std::filesystem::create_directories(ProfilePath());
  sqlite3* db = nullptr;
  if (sqlite3_open(DatabasePath().string().c_str(), &db) != SQLITE_OK) {
    return fallback;
  }
  Execute(db, "CREATE TABLE IF NOT EXISTS settings(key TEXT PRIMARY KEY, value TEXT NOT NULL)");
  sqlite3_stmt* statement = nullptr;
  sqlite3_prepare_v2(db, "SELECT value FROM settings WHERE key=?", -1, &statement, nullptr);
  sqlite3_bind_text(statement, 1, key.c_str(), static_cast<int>(key.size()), SQLITE_TRANSIENT);
  std::string value = fallback;
  if (sqlite3_step(statement) == SQLITE_ROW) {
    const unsigned char* text = sqlite3_column_text(statement, 0);
    value = text ? reinterpret_cast<const char*>(text) : fallback;
  }
  sqlite3_finalize(statement);
  sqlite3_close(db);
  return value;
}

UiText TextFor(const std::string& lang) {
  if (lang == "ja") {
    return {"一般", "外観", "検索", "プライバシー", "Fubukiについて", "設定", "ホームページ", "起動時", "ダウンロード先",
            "テーマ", "言語", "新規タブ背景", "背景色", "背景画像URL", "保存", "保存しました", "検索エンジン", "カスタム検索URL",
            "権限", "このアルファ版では権限プロンプトは標準で拒否されます。履歴、ブックマーク、ダウンロード、設定はローカルの SQLite に保存されます。",
            "バージョン", "エンジン", "プロファイル", "バンドル", "検索またはURLを入力"};
  }
  return {"General", "Appearance", "Search", "Privacy", "About", "Settings", "Homepage", "Startup", "Download folder",
          "Theme", "Language", "New tab background", "Background color", "Background image URL", "Save", "Saved",
          "Search engine", "Custom search URL", "Permissions",
          "Permission prompts are denied by default in this alpha build. History, bookmarks, downloads, and settings are stored locally in SQLite.",
          "Version", "Engine", "Profile", "Bundle", "Search or enter URL"};
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

std::string Selected(bool value) {
  return value ? " selected" : "";
}

std::string NavItem(const std::string& page, const std::string& active, const std::string& icon, const std::string& label) {
  return "<a class=\"" + std::string(page == active ? "active" : "") + "\" href=\"fubuki://settings/" + page +
         "\"><span>" + icon + "</span><strong>" + HtmlEscape(label) + "</strong></a>";
}

std::string LogsHtml() {
  std::filesystem::create_directories(ProfilePath());
  sqlite3* db = nullptr;
  std::ostringstream out;
  out << "<section><h1>Logs</h1>";
  if (sqlite3_open(DatabasePath().string().c_str(), &db) != SQLITE_OK) {
    return out.str() + "<p>Unable to open log store.</p></section>";
  }
  Execute(db, "CREATE TABLE IF NOT EXISTS logs(id INTEGER PRIMARY KEY AUTOINCREMENT,level TEXT NOT NULL,message TEXT NOT NULL,created_at TEXT NOT NULL)");
  sqlite3_stmt* statement = nullptr;
  sqlite3_prepare_v2(db, "SELECT level,message,created_at FROM logs ORDER BY id DESC LIMIT 120", -1, &statement, nullptr);
  out << "<div class=\"log-list\">";
  bool hasRows = false;
  while (sqlite3_step(statement) == SQLITE_ROW) {
    hasRows = true;
    const unsigned char* level = sqlite3_column_text(statement, 0);
    const unsigned char* message = sqlite3_column_text(statement, 1);
    const unsigned char* created = sqlite3_column_text(statement, 2);
    out << "<article><span>" << HtmlEscape(level ? reinterpret_cast<const char*>(level) : "")
        << "</span><strong>" << HtmlEscape(message ? reinterpret_cast<const char*>(message) : "")
        << "</strong><small>" << HtmlEscape(created ? reinterpret_cast<const char*>(created) : "") << "</small></article>";
  }
  if (!hasRows) {
    out << "<p>No logs</p>";
  }
  out << "</div></section>";
  sqlite3_finalize(statement);
  sqlite3_close(db);
  return out.str();
}

std::string NewTabHtml() {
  const std::string lang = Setting("language", "en");
  const UiText t = TextFor(lang);
  const std::string mode = Setting("newTabBackgroundMode", "unsplash");
  const std::string color = Setting("newTabBackgroundColor", "#f8fafd");
  const std::string customUrl = Setting("newTabBackgroundUrl", "");
  const std::string image = mode == "custom" && !customUrl.empty() ? customUrl : kUnsplashSnow;
  const bool useImage = mode != "solid";

  std::ostringstream html;
  html << R"(<!doctype html><html><head><meta charset="utf-8"><title>New Tab</title>)" << FubukiFaviconLink() << R"(<style>
*{box-sizing:border-box}html,body{height:100%}body{margin:0;font:15px -apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;color:#fff;background:)"
       << HtmlEscape(color) << R"(;display:grid;place-items:center;overflow:hidden}
body::before{content:"";position:fixed;inset:0;background:)"
       << (useImage ? "linear-gradient(180deg,rgb(11 18 32/.18),rgb(11 18 32/.42)),url('" + HtmlEscape(image) + "') center/cover" : "transparent")
       << R"(;transform:scale(1.02)}
main{position:relative;width:min(780px,calc(100vw - 48px));display:grid;gap:28px;justify-items:center;text-align:center;animation:page-in .12s linear both}
.brand{display:flex;align-items:center;justify-content:center;gap:18px;text-shadow:0 8px 28px rgb(0 0 0/.26)}.logo{width:76px;height:76px;filter:drop-shadow(0 18px 34px rgb(0 0 0/.24))}
h1{font-size:clamp(36px,6vw,66px);line-height:1;margin:0;font-weight:760;letter-spacing:0}
form{width:min(640px,100%)}input{width:100%;height:54px;border:0;border-radius:27px;padding:0 22px;background:rgb(255 255 255/.9);color:#111827;font:inherit;font-size:17px;outline:0;box-shadow:0 18px 42px rgb(0 0 0/.22);transition:box-shadow .08s linear}
input:focus{background:#fff;box-shadow:0 20px 48px rgb(0 0 0/.28)}
@keyframes page-in{from{opacity:0}}
</style></head><body><main><div class="brand">)"
       << FubukiLogoSvg() << R"(<h1>Fubuki Browser Alpha</h1></div>
<form action="fubuki://newtab/search" method="get"><input name="q" autocomplete="off" autofocus placeholder=")"
       << HtmlEscape(t.searchPlaceholder) << R"("></form></main></body></html>)";
  return html.str();
}

std::string SettingsPageHtml(const std::string& url) {
  const std::string page = PathName(url);
  const std::string lang = Setting("language", "en");
  const UiText t = TextFor(lang);
  const std::string homepage = Setting("homepage", "https://example.com");
  const std::string appearance = Setting("appearance", "system");
  const std::string toolbarDensity = Setting("toolbarDensity", "compact");
  const std::string sidebarVisible = Setting("sidebarVisible", "show");
  const std::string sidebarWidth = Setting("sidebarWidth", "196");
  const std::string openBookmarkIn = Setting("openBookmarkIn", "current");
  const std::string showBookmarkFavicons = Setting("showBookmarkFavicons", "on");
  const std::string searchEngine = Setting("searchEngine", "google");
  const std::string customSearchUrl = Setting("customSearchUrl", "https://www.google.com/search?q={query}");
  std::string startupBehavior = Setting("startupBehavior", "newTab");
  if (startupBehavior == "homepage") {
    startupBehavior = "newTab";
  }
  const std::string newTabPage = Setting("newTabPage", "blank");
  const std::string homeUrl = Setting("homeUrl", homepage);
  const std::string downloadDirectory = Setting("downloadDirectory", "");
  const std::string askBeforeDownload = Setting("askBeforeDownload", "off");
  const std::string bgMode = Setting("newTabBackgroundMode", "unsplash");
  const std::string bgColor = Setting("newTabBackgroundColor", "#f8fafd");
  const std::string bgUrl = Setting("newTabBackgroundUrl", "");
  const std::string currentTabLabel = lang == "ja" ? "現在のタブ" : "Current tab";
  const std::string newWindowTabLabel = lang == "ja" ? "新しいタブ" : "New tab";
  const std::string blankLabel = lang == "ja" ? "空白" : "Blank";
  const std::string homeLabel = lang == "ja" ? "ホーム" : "Home";
  const std::string newTabLabel = lang == "ja" ? "新規タブ" : "New tab";
  const std::string restoreLabel = lang == "ja" ? "前回のセッション" : "Restore last session";
  const std::string compactLabel = lang == "ja" ? "コンパクト" : "Compact";
  const std::string comfortableLabel = lang == "ja" ? "標準" : "Comfortable";
  const std::string showLabel = lang == "ja" ? "表示" : "Show";
  const std::string hideLabel = lang == "ja" ? "非表示" : "Hide";
  const std::string collapsedLabel = lang == "ja" ? "折りたたみ" : "Collapsed";
  const std::string onLabel = lang == "ja" ? "オン" : "On";
  const std::string offLabel = lang == "ja" ? "オフ" : "Off";
  const std::string unsplashLabel = lang == "ja" ? "吹雪写真" : "Unsplash";
  const std::string customLabel = lang == "ja" ? "カスタムURL" : "Custom URL";
  const std::string solidLabel = lang == "ja" ? "単色" : "Solid";
  const std::string systemLabel = lang == "ja" ? "システム" : "System";
  const std::string lightLabel = lang == "ja" ? "ライト" : "Light";
  const std::string darkLabel = lang == "ja" ? "ダーク" : "Dark";
  const std::string toolbarDensityLabel = lang == "ja" ? "ツールバー密度" : "Toolbar density";
  const std::string sidebarLabel = lang == "ja" ? "サイドバー" : "Sidebar";
  const std::string sidebarWidthLabel = lang == "ja" ? "サイドバー幅" : "Sidebar width";
  const std::string bookmarksLabel = lang == "ja" ? "ブックマーク" : "Bookmarks";
  const std::string tabsLabel = lang == "ja" ? "タブ" : "Tabs";
  const std::string downloadsLabel = lang == "ja" ? "ダウンロード" : "Downloads";
  const std::string privacyDataLabel = lang == "ja" ? "プライバシーとデータ" : "Privacy & Data";
  const std::string advancedLabel = lang == "ja" ? "詳細" : "Advanced";
  const std::string openBookmarkInLabel = lang == "ja" ? "ブックマークを開く先" : "Open bookmark in";
  const std::string showFaviconLabel = lang == "ja" ? "ブックマークにfaviconを表示" : "Show favicon in bookmarks";
  const std::string newTabPageLabel = lang == "ja" ? "新規タブページ" : "New tab page";
  const std::string askBeforeDownloadLabel = lang == "ja" ? "ダウンロード前に確認" : "Ask before download";
  const std::string clearDownloadHistoryLabel = lang == "ja" ? "ダウンロード履歴を消去" : "Clear download history";
  const std::string clearDownloadsLabel = lang == "ja" ? "ダウンロードを消去" : "Clear downloads";
  const std::string customSearchLabel = lang == "ja" ? "カスタム" : "Custom";
  const std::string clearHistoryLabel = lang == "ja" ? "履歴を消去" : "Clear history";
  const std::string clearBookmarksLabel = lang == "ja" ? "ブックマークを消去" : "Clear bookmarks";
  const std::string clearAllLocalDataLabel = lang == "ja" ? "すべてのローカルデータを消去" : "Clear all local data";
  const std::string clearAllLabel = lang == "ja" ? "すべて消去" : "Clear all";
  const std::string devToolsLabel = "DevTools";
  const std::string openDevToolsLabel = lang == "ja" ? "DevToolsを開く" : "Open DevTools";
  const std::string logsLabel = lang == "ja" ? "ログ" : "Logs";
  const std::string viewLogsLabel = lang == "ja" ? "ログを表示" : "View logs";
  const std::string clearLogsLabel = lang == "ja" ? "ログを消去" : "Clear logs";
  const std::string homeUrlLabel = lang == "ja" ? "ホームURL" : "Home URL";
  const std::string cefVersionLabel = lang == "ja" ? "CEFバージョン" : "CEF version";

  auto settingLink = [&](const std::string& key, const std::string& value, const std::string& returnPage) {
    return "fubuki://settings/set?key=" + key + "&value=" + CefURIEncode(value, false).ToString() + "&return=" + returnPage;
  };

  std::ostringstream content;
  if (page == "appearance") {
    content << "<section><h1>" << HtmlEscape(t.appearance) << "</h1><div class=\"field\"><span>" << HtmlEscape(t.theme)
            << "</span><div class=\"segmented\">"
            << "<a class=\"chip" << Selected(appearance == "system") << "\" href=\"" << settingLink("appearance", "system", "appearance") << "\">" << HtmlEscape(systemLabel) << "</a>"
            << "<a class=\"chip" << Selected(appearance == "light") << "\" href=\"" << settingLink("appearance", "light", "appearance") << "\">" << HtmlEscape(lightLabel) << "</a>"
            << "<a class=\"chip" << Selected(appearance == "dark") << "\" href=\"" << settingLink("appearance", "dark", "appearance") << "\">" << HtmlEscape(darkLabel) << "</a></div></div>"
            << "<div class=\"field\"><span>" << HtmlEscape(toolbarDensityLabel) << "</span><div class=\"segmented\">"
            << "<a class=\"chip" << Selected(toolbarDensity == "compact") << "\" href=\"" << settingLink("toolbarDensity", "compact", "appearance") << "\">" << HtmlEscape(compactLabel) << "</a>"
            << "<a class=\"chip" << Selected(toolbarDensity == "comfortable") << "\" href=\"" << settingLink("toolbarDensity", "comfortable", "appearance") << "\">" << HtmlEscape(comfortableLabel) << "</a></div></div>"
            << "<div class=\"field\"><span>" << HtmlEscape(sidebarLabel) << "</span><div class=\"segmented\">"
            << "<a class=\"chip" << Selected(sidebarVisible == "show") << "\" href=\"" << settingLink("sidebarVisible", "show", "appearance") << "\">" << HtmlEscape(showLabel) << "</a>"
            << "<a class=\"chip" << Selected(sidebarVisible == "collapsed") << "\" href=\"" << settingLink("sidebarVisible", "collapsed", "appearance") << "\">" << HtmlEscape(collapsedLabel) << "</a>"
            << "<a class=\"chip" << Selected(sidebarVisible == "hide") << "\" href=\"" << settingLink("sidebarVisible", "hide", "appearance") << "\">" << HtmlEscape(hideLabel) << "</a></div></div>"
            << "<form action=\"fubuki://settings/set\" method=\"get\"><input type=\"hidden\" name=\"key\" value=\"sidebarWidth\"><input type=\"hidden\" name=\"return\" value=\"appearance\"><label>" << HtmlEscape(sidebarWidthLabel)
            << "<input name=\"value\" type=\"number\" min=\"160\" max=\"240\" value=\"" << HtmlEscape(sidebarWidth) << "\"></label><button>" << HtmlEscape(t.save) << "</button></form>"
            << "<div class=\"field\"><span>" << HtmlEscape(t.language) << "</span><div class=\"segmented\">"
            << "<a class=\"chip" << Selected(lang == "en") << "\" href=\"" << settingLink("language", "en", "appearance") << "\">English</a>"
            << "<a class=\"chip" << Selected(lang == "ja") << "\" href=\"" << settingLink("language", "ja", "appearance") << "\">日本語</a></div></div>"
            << "<div class=\"field\"><span>" << HtmlEscape(t.newTabBackground) << "</span><div class=\"segmented\">"
            << "<a class=\"chip" << Selected(bgMode == "unsplash") << "\" href=\"" << settingLink("newTabBackgroundMode", "unsplash", "appearance") << "\">" << HtmlEscape(unsplashLabel) << "</a>"
            << "<a class=\"chip" << Selected(bgMode == "custom") << "\" href=\"" << settingLink("newTabBackgroundMode", "custom", "appearance") << "\">" << HtmlEscape(customLabel) << "</a>"
            << "<a class=\"chip" << Selected(bgMode == "solid") << "\" href=\"" << settingLink("newTabBackgroundMode", "solid", "appearance") << "\">" << HtmlEscape(solidLabel) << "</a></div></div>"
            << "<form action=\"fubuki://settings/set\" method=\"get\"><input type=\"hidden\" name=\"key\" value=\"newTabBackgroundColor\"><input type=\"hidden\" name=\"return\" value=\"appearance\"><label>" << HtmlEscape(t.backgroundColor)
            << "<input name=\"value\" value=\"" << HtmlEscape(bgColor) << "\"></label><button>" << HtmlEscape(t.save) << "</button></form>"
            << "<form action=\"fubuki://settings/set\" method=\"get\"><input type=\"hidden\" name=\"key\" value=\"newTabBackgroundUrl\"><input type=\"hidden\" name=\"return\" value=\"appearance\"><label>" << HtmlEscape(t.backgroundUrl)
            << "<input name=\"value\" value=\"" << HtmlEscape(bgUrl) << "\"></label><button>" << HtmlEscape(t.save) << "</button></form></section>";
  } else if (page == "bookmarks") {
    content << "<section><h1>" << HtmlEscape(bookmarksLabel) << "</h1>"
            << "<div class=\"field\"><span>" << HtmlEscape(openBookmarkInLabel) << "</span><div class=\"segmented\">"
            << "<a class=\"chip" << Selected(openBookmarkIn == "current") << "\" href=\"" << settingLink("openBookmarkIn", "current", "bookmarks") << "\">" << HtmlEscape(currentTabLabel) << "</a>"
            << "<a class=\"chip" << Selected(openBookmarkIn == "new") << "\" href=\"" << settingLink("openBookmarkIn", "new", "bookmarks") << "\">" << HtmlEscape(newWindowTabLabel) << "</a></div></div>"
            << "<div class=\"field\"><span>" << HtmlEscape(showFaviconLabel) << "</span><div class=\"segmented\">"
            << "<a class=\"chip" << Selected(showBookmarkFavicons == "on") << "\" href=\"" << settingLink("showBookmarkFavicons", "on", "bookmarks") << "\">" << HtmlEscape(onLabel) << "</a>"
            << "<a class=\"chip" << Selected(showBookmarkFavicons == "off") << "\" href=\"" << settingLink("showBookmarkFavicons", "off", "bookmarks") << "\">" << HtmlEscape(offLabel) << "</a></div></div></section>";
  } else if (page == "tabs") {
    content << "<section><h1>" << HtmlEscape(tabsLabel) << "</h1>"
            << "<div class=\"field\"><span>" << HtmlEscape(newTabPageLabel) << "</span><div class=\"segmented\">"
            << "<a class=\"chip" << Selected(newTabPage == "blank") << "\" href=\"" << settingLink("newTabPage", "blank", "tabs") << "\">" << HtmlEscape(blankLabel) << "</a>"
            << "<a class=\"chip" << Selected(newTabPage == "home") << "\" href=\"" << settingLink("newTabPage", "home", "tabs") << "\">" << HtmlEscape(homeLabel) << "</a></div></div></section>";
  } else if (page == "downloads") {
    content << "<section><h1>" << HtmlEscape(downloadsLabel) << "</h1>"
            << "<form action=\"fubuki://settings/set\" method=\"get\"><input type=\"hidden\" name=\"key\" value=\"downloadDirectory\"><input type=\"hidden\" name=\"return\" value=\"downloads\"><label>" << HtmlEscape(t.downloadFolder)
            << "<input name=\"value\" value=\"" << HtmlEscape(downloadDirectory) << "\"></label><button>" << HtmlEscape(t.save) << "</button></form>"
            << "<div class=\"field\"><span>" << HtmlEscape(askBeforeDownloadLabel) << "</span><div class=\"segmented\">"
            << "<a class=\"chip" << Selected(askBeforeDownload == "on") << "\" href=\"" << settingLink("askBeforeDownload", "on", "downloads") << "\">" << HtmlEscape(onLabel) << "</a>"
            << "<a class=\"chip" << Selected(askBeforeDownload == "off") << "\" href=\"" << settingLink("askBeforeDownload", "off", "downloads") << "\">" << HtmlEscape(offLabel) << "</a></div></div>"
            << "<div class=\"field\"><span>" << HtmlEscape(clearDownloadHistoryLabel) << "</span><div class=\"segmented\"><a class=\"chip danger\" href=\"" << settingLink("clearData", "downloads", "downloads") << "\">" << HtmlEscape(clearDownloadsLabel) << "</a></div></div></section>";
  } else if (page == "search") {
    content << "<section><h1>" << HtmlEscape(t.search) << "</h1><div class=\"field\"><span>" << HtmlEscape(t.engine)
            << "</span><div class=\"segmented\">"
            << "<a class=\"chip" << Selected(searchEngine == "duckduckgo") << "\" href=\"" << settingLink("searchEngine", "duckduckgo", "search") << "\">DuckDuckGo</a>"
            << "<a class=\"chip" << Selected(searchEngine == "google") << "\" href=\"" << settingLink("searchEngine", "google", "search") << "\">Google</a>"
            << "<a class=\"chip" << Selected(searchEngine == "bing") << "\" href=\"" << settingLink("searchEngine", "bing", "search") << "\">Bing</a>"
            << "<a class=\"chip" << Selected(searchEngine == "custom") << "\" href=\"" << settingLink("searchEngine", "custom", "search") << "\">" << HtmlEscape(customSearchLabel) << "</a></div></div>"
            << "<form action=\"fubuki://settings/set\" method=\"get\"><input type=\"hidden\" name=\"key\" value=\"customSearchUrl\"><input type=\"hidden\" name=\"return\" value=\"search\"><label>" << HtmlEscape(t.customSearchUrl)
            << "<input name=\"value\" value=\"" << HtmlEscape(customSearchUrl) << "\" placeholder=\"https://example.com/search?q={query}\"></label><button>" << HtmlEscape(t.save) << "</button></form></section>";
  } else if (page == "privacy") {
    content << "<section><h1>" << HtmlEscape(privacyDataLabel) << "</h1><div class=\"hero-line\"></div><h2>" << HtmlEscape(t.permissions)
            << "</h2><p>" << HtmlEscape(t.privacyBody) << "</p>"
            << "<div class=\"field\"><span>" << HtmlEscape(clearHistoryLabel) << "</span><div class=\"segmented\"><a class=\"chip danger\" href=\"" << settingLink("clearData", "history", "privacy") << "\">" << HtmlEscape(clearHistoryLabel) << "</a></div></div>"
            << "<div class=\"field\"><span>" << HtmlEscape(clearBookmarksLabel) << "</span><div class=\"segmented\"><a class=\"chip danger\" href=\"" << settingLink("clearData", "bookmarks", "privacy") << "\">" << HtmlEscape(clearBookmarksLabel) << "</a></div></div>"
            << "<div class=\"field\"><span>" << HtmlEscape(clearDownloadHistoryLabel) << "</span><div class=\"segmented\"><a class=\"chip danger\" href=\"" << settingLink("clearData", "downloads", "privacy") << "\">" << HtmlEscape(clearDownloadsLabel) << "</a></div></div>"
            << "<div class=\"field\"><span>" << HtmlEscape(clearAllLocalDataLabel) << "</span><div class=\"segmented\"><a class=\"chip danger\" href=\"" << settingLink("clearData", "all", "privacy") << "\">" << HtmlEscape(clearAllLabel) << "</a></div></div></section>";
  } else if (page == "advanced") {
    content << "<section><h1>" << HtmlEscape(advancedLabel) << "</h1>"
            << "<div class=\"field\"><span>" << HtmlEscape(devToolsLabel) << "</span><div class=\"segmented\"><a class=\"chip\" href=\"" << settingLink("openDevTools", "1", "advanced") << "\">" << HtmlEscape(openDevToolsLabel) << "</a></div></div>"
            << "<div class=\"field\"><span>" << HtmlEscape(logsLabel) << "</span><div class=\"segmented\"><a class=\"chip\" href=\"fubuki://settings/logs\">" << HtmlEscape(viewLogsLabel) << "</a>"
            << "<a class=\"chip danger\" href=\"" << settingLink("clearData", "logs", "advanced") << "\">" << HtmlEscape(clearLogsLabel) << "</a></div></div></section>";
  } else if (page == "logs") {
    content << LogsHtml();
  } else if (page == "about") {
    content << "<section class=\"about-page\">" << FubukiLogoSvg("about-logo")
            << "<h1>Fubuki Browser Alpha</h1>"
            << "<div class=\"about-grid\"><div><span>" << HtmlEscape(t.version) << "</span><strong>0.1.0 alpha</strong></div>"
            << "<div><span>" << HtmlEscape(cefVersionLabel) << "</span><strong>" << HtmlEscape(CEF_VERSION) << "</strong></div>"
            << "<div><span>" << HtmlEscape(lang == "ja" ? "権利表示" : "Copyright") << "</span><strong>Copyright 2026 TeamFubuki<br>Released under the MIT license</strong></div></div></section>";
  } else {
    content << "<section><h1>" << HtmlEscape(t.general) << "</h1>"
            << "<div class=\"field\"><span>" << HtmlEscape(t.startup) << "</span><div class=\"segmented\">"
            << "<a class=\"chip" << Selected(startupBehavior == "newTab") << "\" href=\"" << settingLink("startupBehavior", "newTab", "general") << "\">" << HtmlEscape(newTabLabel) << "</a>"
            << "<a class=\"chip" << Selected(startupBehavior == "restore") << "\" href=\"" << settingLink("startupBehavior", "restore", "general") << "\">" << HtmlEscape(restoreLabel) << "</a></div></div>"
            << "<form action=\"fubuki://settings/set\" method=\"get\"><input type=\"hidden\" name=\"key\" value=\"homeUrl\"><input type=\"hidden\" name=\"return\" value=\"general\"><label>" << HtmlEscape(homeUrlLabel)
            << "<input name=\"value\" value=\"" << HtmlEscape(homeUrl) << "\"></label><button>" << HtmlEscape(t.save) << "</button></form></section>";
  }

  std::ostringstream html;
  html << R"(<!doctype html><html data-appearance=")" << HtmlEscape(appearance) << R"("><head><meta charset="utf-8"><title>)" << HtmlEscape(t.settings) << R"(</title>)" << FubukiFaviconLink() << R"(<style>
*{box-sizing:border-box}body{margin:0;background:var(--bg);color:var(--text);font:14px -apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;letter-spacing:0;user-select:none;--bg:#f6f7fb;--surface:#fff;--panel:#eef2f7;--text:#161a20;--muted:#596473;--line:rgb(25 34 48/.12);--shadow:0 18px 48px rgb(25 34 48/.10);--accent:#2563eb}
html[data-appearance=dark] body{--bg:#16191f;--surface:#20242c;--panel:#111419;--text:#f4f7fb;--muted:#a7b0bd;--line:rgb(255 255 255/.12);--shadow:0 12px 30px rgb(0 0 0/.22);--accent:#8fb8ff;color-scheme:dark}
@media(prefers-color-scheme:dark){html[data-appearance=system] body{--bg:#16191f;--surface:#20242c;--panel:#111419;--text:#f4f7fb;--muted:#a7b0bd;--line:rgb(255 255 255/.12);--shadow:0 12px 30px rgb(0 0 0/.22);--accent:#8fb8ff;color-scheme:dark}}
.settings{min-height:100vh;display:grid;grid-template-columns:220px minmax(0,1fr);animation:page-in .1s linear both}
aside{position:sticky;top:0;height:100vh;padding:22px 12px;background:color-mix(in srgb,var(--panel) 92%,transparent);border-right:1px solid var(--line);backdrop-filter:blur(18px) saturate(1.1)}
.brand{height:48px;display:flex;align-items:center;gap:10px;padding:0 10px;margin-bottom:18px}.logo{width:34px;height:34px}.brand strong{font-size:16px}
nav{display:grid;gap:4px}nav a{height:42px;display:flex;align-items:center;gap:12px;padding:0 12px;border-radius:10px;color:var(--muted);text-decoration:none;transition:background-color .08s linear,color .08s linear}nav a:hover{background:color-mix(in srgb,var(--surface) 70%,transparent)}nav a span{width:22px;text-align:center;font-size:17px}nav a strong{font-weight:650}nav a.active{background:var(--surface);color:var(--text);box-shadow:var(--shadow)}
main{padding:42px clamp(26px,5vw,70px);display:grid;align-content:start}section{max-width:760px;display:grid;gap:18px}h1{font-size:34px;line-height:1;margin:0;font-weight:760}h2{font-size:18px;margin:0}.field,form{display:grid;gap:10px;background:var(--surface);border:1px solid var(--line);border-radius:14px;padding:16px;box-shadow:var(--shadow);transition:border-color .08s linear}.field:hover,form:hover{border-color:color-mix(in srgb,var(--accent) 28%,var(--line))}
label{display:grid;gap:10px;color:var(--muted);font-weight:650}input{height:38px;border:1px solid var(--line);border-radius:9px;background:var(--panel);padding:0 12px;color:var(--text);font:inherit;user-select:text;transition:border-color .08s linear,box-shadow .08s linear}input:focus{outline:0;border-color:var(--accent);box-shadow:0 0 0 3px color-mix(in srgb,var(--accent) 22%,transparent);background:var(--surface)}
button,.chip{height:34px;border:0;border-radius:9px;background:var(--panel);color:var(--text);padding:0 12px;text-decoration:none;font:inherit;font-weight:700;display:inline-grid;place-items:center;transition:background-color .08s linear,color .08s linear}.segmented{display:flex;flex-wrap:wrap;gap:8px}.chip.selected,button{background:var(--text);color:var(--bg)}.chip.danger{background:#fee2e2;color:#991b1b}html[data-appearance=dark] .chip.danger{background:#4a1d22;color:#ffb4bd}.hero-line{width:86px;height:6px;border-radius:999px;background:linear-gradient(90deg,#ff9686,#a7abe0,#1aadeb)}
.about-page{max-width:760px;min-height:calc(100vh - 84px);place-content:center}.about-logo{width:112px;height:112px}.about-page h1{font-size:clamp(40px,7vw,72px);letter-spacing:0}.about-grid{margin-top:18px;display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:12px}.about-grid div{background:var(--surface);border:1px solid var(--line);border-radius:14px;padding:16px;box-shadow:var(--shadow)}.about-grid div:last-child{grid-column:1/-1}.about-grid span{display:block;color:var(--muted);font-size:12px;margin-bottom:6px}.about-grid strong{word-break:break-word}
.log-list{display:grid;gap:8px}.log-list article{display:grid;grid-template-columns:76px minmax(0,1fr);gap:4px 10px;background:var(--surface);border:1px solid var(--line);border-radius:10px;padding:10px}.log-list span{grid-row:1/3;color:var(--muted);font-size:12px;text-transform:uppercase}.log-list strong{font-weight:600;word-break:break-word}.log-list small{color:var(--muted);font-size:12px}
@keyframes page-in{from{opacity:0}}
@media(max-width:760px){.settings{grid-template-columns:1fr}aside{position:static;height:auto}.about-grid{grid-template-columns:1fr}}
</style></head><body><div class="settings"><aside><div class="brand">)"
       << FubukiLogoSvg() << "<strong>" << HtmlEscape(t.settings) << "</strong></div><nav>"
       << NavItem("general", page, "⌂", t.general)
       << NavItem("search", page, "⌕", t.search)
       << NavItem("appearance", page, "◐", t.appearance)
       << NavItem("tabs", page, "▣", tabsLabel)
       << NavItem("bookmarks", page, "★", bookmarksLabel)
       << NavItem("downloads", page, "↓", downloadsLabel)
       << NavItem("privacy", page, "◇", privacyDataLabel)
       << NavItem("advanced", page, "⌘", advancedLabel)
       << NavItem("about", page, "ⓘ", t.about)
       << "</nav></aside><main>" << content.str() << "</main></div></body></html>";
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
  if (url.rfind("fubuki://settings", 0) == 0) {
    LoadText(SettingsPageHtml(url), "text/html", 200);
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
