#include "browser/BrowserDataStore.h"

#include <chrono>
#include <cstdlib>
#include <fstream>
#include <iomanip>
#include <sstream>
#include <stdexcept>

#include <sqlite3.h>

#include "include/cef_parser.h"

namespace fubuki {

namespace {

constexpr size_t kMaxHistoryItems = 500;
constexpr size_t kMaxDownloadItems = 20;
constexpr size_t kMaxLogItems = 300;

std::string ColumnText(sqlite3_stmt *statement, int column) {
  const unsigned char *text = sqlite3_column_text(statement, column);
  return text ? reinterpret_cast<const char *>(text) : "";
}

void BindText(sqlite3_stmt *statement, int index, const std::string &value) {
  sqlite3_bind_text(statement, index, value.c_str(),
                    static_cast<int>(value.size()), SQLITE_TRANSIENT);
}

}  // namespace

BrowserDataStore::BrowserDataStore(std::filesystem::path profilePath)
    : profilePath_(std::move(profilePath)),
      databasePath_(profilePath_ / "fubuki.sqlite3"),
      history_(CefListValue::Create()), bookmarks_(CefListValue::Create()),
      downloads_(CefListValue::Create()), permissions_(CefListValue::Create()),
      settings_(CefDictionaryValue::Create()), logs_(CefListValue::Create()) {}

BrowserDataStore::~BrowserDataStore() {
  if (db_) {
    sqlite3_close(db_);
    db_ = nullptr;
  }
}

void BrowserDataStore::Load() {
  std::filesystem::create_directories(profilePath_);
  OpenDatabase();
  EnsureSchema();
  MigrateJsonFiles();
  EnsureDefaultSetting("homepage", "https://example.com");
  EnsureDefaultSetting("searchEngine", "google");
  EnsureDefaultSetting("customSearchUrl",
                       "https://www.google.com/search?q={query}");
  EnsureDefaultSetting("startupBehavior", "newTab");
  EnsureDefaultSetting("sessionJson", "");
  const char *home = std::getenv("HOME");
  EnsureDefaultSetting(
      "downloadDirectory",
      home ? (std::filesystem::path(home) / "Downloads").string() : "/tmp");
  EnsureDefaultSetting("theme", "light");
  EnsureDefaultSetting("appearance", "system");
  EnsureDefaultSetting("toolbarDensity", "compact");
  EnsureDefaultSetting("sidebarVisible", "show");
  EnsureDefaultSetting("sidebarWidth", "196");
  EnsureDefaultSetting("defaultBookmarkDisplay", "sidebar");
  EnsureDefaultSetting("openBookmarkIn", "current");
  EnsureDefaultSetting("showBookmarkFavicons", "on");
  EnsureDefaultSetting("newTabPage", "blank");
  EnsureDefaultSetting("homeUrl", "https://example.com");
  EnsureDefaultSetting("askBeforeDownload", "off");
  EnsureDefaultSetting("defaultZoomLevel", "0");
  EnsureDefaultSetting("closeWindowWithLastTab", "off");
  EnsureDefaultSetting("privateSearchEngine", "default");
  EnsureDefaultSetting("language", "system");
  EnsureDefaultSetting("newTabBackgroundMode", "unsplash");
  EnsureDefaultSetting("newTabBackgroundColor", "#f8fafd");
  EnsureDefaultSetting("newTabBackgroundUrl", "");
  RefreshCaches();
}

void BrowserDataStore::AddHistory(const std::string &title,
                                  const std::string &url) {
  if (url.empty() || url.rfind("fubuki://", 0) == 0 ||
      url.rfind("data:", 0) == 0) {
    return;
  }
  sqlite3_stmt *statement = nullptr;
  sqlite3_prepare_v2(db_,
                     "INSERT INTO history(title,url,created_at) VALUES(?,?,?)",
                     -1, &statement, nullptr);
  BindText(statement, 1, title.empty() ? url : title);
  BindText(statement, 2, url);
  BindText(statement, 3, NowIsoString());
  sqlite3_step(statement);
  sqlite3_finalize(statement);
  Execute("DELETE FROM history WHERE id NOT IN (SELECT id FROM history ORDER "
          "BY id DESC LIMIT 500)");
  RefreshList("history", history_, kMaxHistoryItems);
}

bool BrowserDataStore::AddBookmark(const std::string &title,
                                   const std::string &url,
                                   const std::string &faviconUrl) {
  if (url.empty() || url.rfind("fubuki://", 0) == 0 ||
      url.rfind("data:", 0) == 0) {
    return false;
  }
  sqlite3_stmt *statement = nullptr;
  sqlite3_prepare_v2(
      db_,
      "INSERT INTO bookmarks(title,url,favicon_url,created_at) VALUES(?,?,?,?) "
      "ON CONFLICT(url) DO UPDATE SET "
      "title=excluded.title,favicon_url=excluded.favicon_url",
      -1, &statement, nullptr);
  BindText(statement, 1, title.empty() ? url : title);
  BindText(statement, 2, url);
  BindText(statement, 3, faviconUrl);
  BindText(statement, 4, NowIsoString());
  const bool ok = sqlite3_step(statement) == SQLITE_DONE;
  sqlite3_finalize(statement);
  RefreshList("bookmarks", bookmarks_, 500);
  return ok;
}

bool BrowserDataStore::RemoveBookmark(const std::string &url) {
  sqlite3_stmt *statement = nullptr;
  sqlite3_prepare_v2(db_, "DELETE FROM bookmarks WHERE url=?", -1, &statement,
                     nullptr);
  BindText(statement, 1, url);
  const bool ok =
      sqlite3_step(statement) == SQLITE_DONE && sqlite3_changes(db_) > 0;
  sqlite3_finalize(statement);
  RefreshList("bookmarks", bookmarks_, 500);
  return ok;
}

bool BrowserDataStore::RemoveHistory(const std::string &url) {
  sqlite3_stmt *statement = nullptr;
  sqlite3_prepare_v2(db_, "DELETE FROM history WHERE url=?", -1, &statement,
                     nullptr);
  BindText(statement, 1, url);
  const bool ok =
      sqlite3_step(statement) == SQLITE_DONE && sqlite3_changes(db_) > 0;
  sqlite3_finalize(statement);
  RefreshList("history", history_, kMaxHistoryItems);
  return ok;
}

bool BrowserDataStore::RemoveDownload(const std::string &url,
                                      const std::string &path) {
  sqlite3_stmt *statement = nullptr;
  sqlite3_prepare_v2(
      db_,
      "DELETE FROM downloads WHERE (url=? AND ?<>'') OR (path=? AND ?<>'')", -1,
      &statement, nullptr);
  BindText(statement, 1, url);
  BindText(statement, 2, url);
  BindText(statement, 3, path);
  BindText(statement, 4, path);
  const bool ok =
      sqlite3_step(statement) == SQLITE_DONE && sqlite3_changes(db_) > 0;
  sqlite3_finalize(statement);
  RefreshList("downloads", downloads_, kMaxDownloadItems);
  return ok;
}

bool BrowserDataStore::HasDownloadPath(const std::string &path) const {
  if (path.empty()) {
    return false;
  }
  sqlite3_stmt *statement = nullptr;
  sqlite3_prepare_v2(db_, "SELECT 1 FROM downloads WHERE path=? LIMIT 1", -1,
                     &statement, nullptr);
  BindText(statement, 1, path);
  const bool ok = sqlite3_step(statement) == SQLITE_ROW;
  sqlite3_finalize(statement);
  return ok;
}

bool BrowserDataStore::ClearBookmarks() {
  Execute("DELETE FROM bookmarks");
  RefreshList("bookmarks", bookmarks_, 500);
  return true;
}

bool BrowserDataStore::ClearHistory() {
  Execute("DELETE FROM history");
  RefreshList("history", history_, kMaxHistoryItems);
  return true;
}

bool BrowserDataStore::ClearHistoryRange(const std::string &range) {
  if (range == "all") {
    return ClearHistory();
  }

  auto cutoff = std::chrono::system_clock::now();
  if (range == "lastHour") {
    cutoff -= std::chrono::hours(1);
  } else if (range == "today") {
    const auto nowTime = std::chrono::system_clock::to_time_t(cutoff);
    std::tm tm{};
    localtime_r(&nowTime, &tm);
    tm.tm_hour = 0;
    tm.tm_min = 0;
    tm.tm_sec = 0;
    cutoff = std::chrono::system_clock::from_time_t(std::mktime(&tm));
  } else {
    return false;
  }

  const auto cutoffTime = std::chrono::system_clock::to_time_t(cutoff);
  std::tm tm{};
  localtime_r(&cutoffTime, &tm);
  std::ostringstream out;
  out << std::put_time(&tm, "%Y-%m-%dT%H:%M:%S%z");
  const std::string cutoffText = out.str();

  sqlite3_stmt *statement = nullptr;
  sqlite3_prepare_v2(db_, "DELETE FROM history WHERE created_at >= ?", -1,
                     &statement, nullptr);
  BindText(statement, 1, cutoffText);
  const bool ok = sqlite3_step(statement) == SQLITE_DONE;
  sqlite3_finalize(statement);
  RefreshList("history", history_, kMaxHistoryItems);
  return ok;
}

bool BrowserDataStore::ClearDownloads() {
  Execute("DELETE FROM downloads");
  RefreshList("downloads", downloads_, kMaxDownloadItems);
  return true;
}

bool BrowserDataStore::ClearLogs() {
  Execute("DELETE FROM logs");
  RefreshList("logs", logs_, kMaxLogItems);
  return true;
}

bool BrowserDataStore::SetPermission(const std::string &origin,
                                     const std::string &permission,
                                     const std::string &value) {
  if (origin.empty() || permission.empty() ||
      (permission != "notifications" && permission != "camera" &&
       permission != "microphone" && permission != "location" &&
       permission != "popups")) {
    return false;
  }
  if (value != "ask" && value != "allow" && value != "deny") {
    return false;
  }
  sqlite3_stmt *statement = nullptr;
  sqlite3_prepare_v2(
      db_,
      "INSERT INTO site_permissions(origin,permission,value,updated_at) "
      "VALUES(?,?,?,?) "
      "ON CONFLICT(origin,permission) DO UPDATE SET "
      "value=excluded.value,updated_at=excluded.updated_at",
      -1, &statement, nullptr);
  BindText(statement, 1, origin);
  BindText(statement, 2, permission);
  BindText(statement, 3, value);
  BindText(statement, 4, NowIsoString());
  const bool ok = sqlite3_step(statement) == SQLITE_DONE;
  sqlite3_finalize(statement);
  RefreshList("site_permissions", permissions_, 500);
  return ok;
}

bool BrowserDataStore::RemovePermission(const std::string &origin,
                                        const std::string &permission) {
  sqlite3_stmt *statement = nullptr;
  sqlite3_prepare_v2(
      db_, "DELETE FROM site_permissions WHERE origin=? AND permission=?", -1,
      &statement, nullptr);
  BindText(statement, 1, origin);
  BindText(statement, 2, permission);
  const bool ok =
      sqlite3_step(statement) == SQLITE_DONE && sqlite3_changes(db_) > 0;
  sqlite3_finalize(statement);
  RefreshList("site_permissions", permissions_, 500);
  return ok;
}

void BrowserDataStore::AddDownload(const std::string &url,
                                   const std::string &path,
                                   const std::string &state) {
  UpdateDownload(url, path, state, 0);
}

void BrowserDataStore::UpdateDownload(const std::string &url,
                                      const std::string &path,
                                      const std::string &state, int percent) {
  const std::string now = NowIsoString();
  sqlite3_stmt *statement = nullptr;
  sqlite3_prepare_v2(db_,
                     "UPDATE downloads SET state=?,percent=?,updated_at=? "
                     "WHERE COALESCE(url,'')=COALESCE(?, '') AND "
                     "COALESCE(path,'')=COALESCE(?, '')",
                     -1, &statement, nullptr);
  BindText(statement, 1, state);
  sqlite3_bind_int(statement, 2, percent);
  BindText(statement, 3, now);
  BindText(statement, 4, url);
  BindText(statement, 5, path);
  sqlite3_step(statement);
  const bool updated = sqlite3_changes(db_) > 0;
  sqlite3_finalize(statement);

  if (!updated) {
    sqlite3_prepare_v2(
        db_,
        "INSERT INTO downloads(url,path,state,percent,created_at,updated_at) "
        "VALUES(?,?,?,?,?,?)",
        -1, &statement, nullptr);
    BindText(statement, 1, url);
    BindText(statement, 2, path);
    BindText(statement, 3, state);
    sqlite3_bind_int(statement, 4, percent);
    BindText(statement, 5, now);
    BindText(statement, 6, now);
    sqlite3_step(statement);
    sqlite3_finalize(statement);
  }

  if (state == "completed" && !path.empty()) {
    sqlite3_prepare_v2(
        db_,
        "DELETE FROM downloads WHERE COALESCE(path,'')=COALESCE(?, '') "
        "AND id NOT IN (SELECT MAX(id) FROM downloads WHERE "
        "COALESCE(path,'')=COALESCE(?, ''))",
        -1, &statement, nullptr);
    BindText(statement, 1, path);
    BindText(statement, 2, path);
    sqlite3_step(statement);
    sqlite3_finalize(statement);
  }
  Execute("DELETE FROM downloads WHERE id NOT IN (SELECT id FROM downloads "
          "ORDER BY id DESC LIMIT 200)");
  RefreshList("downloads", downloads_, kMaxDownloadItems);
}

void BrowserDataStore::Log(const std::string &level,
                           const std::string &message) {
  sqlite3_stmt *statement = nullptr;
  sqlite3_prepare_v2(db_,
                     "INSERT INTO logs(level,message,created_at) VALUES(?,?,?)",
                     -1, &statement, nullptr);
  BindText(statement, 1, level);
  BindText(statement, 2, message);
  BindText(statement, 3, NowIsoString());
  sqlite3_step(statement);
  sqlite3_finalize(statement);
  Execute("DELETE FROM logs WHERE id NOT IN (SELECT id FROM logs ORDER BY id "
          "DESC LIMIT 300)");
  RefreshList("logs", logs_, kMaxLogItems);
}

void BrowserDataStore::SetSetting(const std::string &key,
                                  const std::string &value) {
  sqlite3_stmt *statement = nullptr;
  sqlite3_prepare_v2(db_,
                     "INSERT INTO settings(key,value) VALUES(?,?) ON "
                     "CONFLICT(key) DO UPDATE SET value=excluded.value",
                     -1, &statement, nullptr);
  BindText(statement, 1, key);
  BindText(statement, 2, value);
  sqlite3_step(statement);
  sqlite3_finalize(statement);
  RefreshSettings();
}

void BrowserDataStore::ResetSetting(const std::string &key) {
  const std::string value = DefaultSetting(key);
  if (!value.empty() || key == "sessionJson") {
    SetSetting(key, value);
  }
}

CefRefPtr<CefDictionaryValue> BrowserDataStore::NewRecord() const {
  auto item = CefDictionaryValue::Create();
  item->SetString("createdAt", NowIsoString());
  return item;
}

void BrowserDataStore::OpenDatabase() {
  if (db_) {
    return;
  }
  if (sqlite3_open(databasePath_.string().c_str(), &db_) != SQLITE_OK) {
    throw std::runtime_error("Failed to open SQLite database");
  }
  Execute("PRAGMA journal_mode=WAL");
  Execute("PRAGMA synchronous=NORMAL");
}

void BrowserDataStore::Execute(const std::string &sql) const {
  char *error = nullptr;
  sqlite3_exec(db_, sql.c_str(), nullptr, nullptr, &error);
  if (error) {
    sqlite3_free(error);
  }
}

void BrowserDataStore::EnsureSchema() {
  Execute("CREATE TABLE IF NOT EXISTS settings(key TEXT PRIMARY KEY, value "
          "TEXT NOT NULL)");
  Execute("CREATE TABLE IF NOT EXISTS bookmarks(id INTEGER PRIMARY KEY "
          "AUTOINCREMENT, title TEXT NOT NULL, url TEXT NOT NULL UNIQUE, "
          "favicon_url TEXT, created_at TEXT NOT NULL)");
  Execute("CREATE TABLE IF NOT EXISTS history(id INTEGER PRIMARY KEY "
          "AUTOINCREMENT, title TEXT NOT NULL, url TEXT NOT NULL, created_at "
          "TEXT NOT NULL)");
  Execute("CREATE TABLE IF NOT EXISTS downloads(id INTEGER PRIMARY KEY "
          "AUTOINCREMENT, url TEXT, path TEXT, state TEXT, percent INTEGER "
          "DEFAULT 0, created_at TEXT NOT NULL, updated_at TEXT)");
  {
    bool hasUpdatedAtIndex = false;
    sqlite3_stmt *pragmaStmt = nullptr;
    if (sqlite3_prepare_v2(db_, "PRAGMA table_info(downloads)", -1, &pragmaStmt,
                           nullptr) == SQLITE_OK) {
      while (sqlite3_step(pragmaStmt) == SQLITE_ROW) {
        const char *colName =
            reinterpret_cast<const char *>(sqlite3_column_text(pragmaStmt, 1));
        if (colName && std::string(colName) == "updated_at") {
          hasUpdatedAtIndex = true;
          break;
        }
      }
    }
    sqlite3_finalize(pragmaStmt);
    if (!hasUpdatedAtIndex) {
      Execute("ALTER TABLE downloads ADD COLUMN updated_at TEXT");
    }
  }
  Execute("UPDATE downloads SET updated_at=created_at WHERE updated_at IS NULL "
          "OR updated_at=''");
  Execute("DELETE FROM downloads WHERE id NOT IN (SELECT MAX(id) FROM "
          "downloads GROUP BY COALESCE(url,''), COALESCE(path,''))");
  Execute("CREATE TABLE IF NOT EXISTS logs(id INTEGER PRIMARY KEY "
          "AUTOINCREMENT, level TEXT, message TEXT, created_at TEXT NOT NULL)");
  Execute("CREATE TABLE IF NOT EXISTS site_permissions(origin TEXT NOT NULL, "
          "permission TEXT NOT NULL, value TEXT NOT NULL, updated_at TEXT NOT "
          "NULL, PRIMARY KEY(origin, permission))");
}

void BrowserDataStore::EnsureDefaultSetting(const std::string &key,
                                            const std::string &value) {
  sqlite3_stmt *statement = nullptr;
  sqlite3_prepare_v2(db_,
                     "INSERT OR IGNORE INTO settings(key,value) VALUES(?,?)",
                     -1, &statement, nullptr);
  BindText(statement, 1, key);
  BindText(statement, 2, value);
  sqlite3_step(statement);
  sqlite3_finalize(statement);
}

std::string BrowserDataStore::DefaultSetting(const std::string &key) const {
  if (key == "homepage")
    return "https://example.com";
  if (key == "searchEngine")
    return "google";
  if (key == "customSearchUrl")
    return "https://www.google.com/search?q={query}";
  if (key == "startupBehavior")
    return "newTab";
  if (key == "sessionJson")
    return "";
  if (key == "downloadDirectory") {
    const char *home = std::getenv("HOME");
    return home ? (std::filesystem::path(home) / "Downloads").string() : "/tmp";
  }
  if (key == "theme") return "light";
  if (key == "appearance") return "system";
  if (key == "toolbarDensity") return "compact";
  if (key == "sidebarVisible") return "show";
  if (key == "sidebarWidth") return "196";
  if (key == "defaultBookmarkDisplay") return "sidebar";
  if (key == "openBookmarkIn") return "current";
  if (key == "showBookmarkFavicons") return "on";
  if (key == "newTabPage") return "blank";
  if (key == "homeUrl") return "https://example.com";
  if (key == "askBeforeDownload") return "off";
  if (key == "defaultZoomLevel") return "0";
  if (key == "closeWindowWithLastTab") return "off";
  if (key == "privateSearchEngine") return "default";
  if (key == "language") return "system";
  if (key == "newTabBackgroundMode") return "unsplash";
  if (key == "newTabBackgroundColor") return "#f8fafd";
  if (key == "newTabBackgroundUrl") return "";
  return "";
}

int BrowserDataStore::CountRows(const std::string &table) const {
  sqlite3_stmt *statement = nullptr;
  const std::string sql = "SELECT COUNT(*) FROM " + table;
  sqlite3_prepare_v2(db_, sql.c_str(), -1, &statement, nullptr);
  int count = 0;
  if (sqlite3_step(statement) == SQLITE_ROW) {
    count = sqlite3_column_int(statement, 0);
  }
  sqlite3_finalize(statement);
  return count;
}

void BrowserDataStore::MigrateJsonFiles() {
  if (CountRows("settings") == 0) {
    MigrateSettingsJson(profilePath_ / "settings.json");
  }
  if (CountRows("bookmarks") == 0) {
    MigrateRecordsJson(profilePath_ / "bookmarks.json", "bookmarks");
  }
  if (CountRows("history") == 0) {
    MigrateRecordsJson(profilePath_ / "history.json", "history");
  }
  if (CountRows("downloads") == 0) {
    MigrateRecordsJson(profilePath_ / "downloads.json", "downloads");
  }
  if (CountRows("logs") == 0) {
    MigrateRecordsJson(profilePath_ / "debug-log.json", "logs");
  }
}

void BrowserDataStore::MigrateSettingsJson(const std::filesystem::path &path) {
  std::ifstream file(path, std::ios::binary);
  if (!file) {
    return;
  }
  std::ostringstream buffer;
  buffer << file.rdbuf();
  auto parsed = CefParseJSON(buffer.str(), JSON_PARSER_RFC);
  if (!parsed || parsed->GetType() != VTYPE_DICTIONARY) {
    return;
  }
  auto dict = parsed->GetDictionary();
  CefDictionaryValue::KeyList keys;
  dict->GetKeys(keys);
  sqlite3_stmt *statement = nullptr;
  sqlite3_prepare_v2(db_,
                     "INSERT OR IGNORE INTO settings(key,value) VALUES(?,?)",
                     -1, &statement, nullptr);
  for (const auto &key : keys) {
    if (dict->GetType(key) != VTYPE_STRING) {
      continue;
    }
    sqlite3_reset(statement);
    BindText(statement, 1, key.ToString());
    BindText(statement, 2, dict->GetString(key).ToString());
    sqlite3_step(statement);
  }
  sqlite3_finalize(statement);
}

void BrowserDataStore::MigrateRecordsJson(const std::filesystem::path &path,
                                          const std::string &table) {
  std::ifstream file(path, std::ios::binary);
  if (!file) {
    return;
  }
  std::ostringstream buffer;
  buffer << file.rdbuf();
  auto parsed = CefParseJSON(buffer.str(), JSON_PARSER_RFC);
  if (!parsed || parsed->GetType() != VTYPE_LIST) {
    return;
  }
  auto list = parsed->GetList();
  const std::string sql =
      table == "bookmarks"
          ? "INSERT OR IGNORE INTO bookmarks(title,url,favicon_url,created_at) "
            "VALUES(?,?,?,?)"
      : table == "history"
          ? "INSERT INTO history(title,url,created_at) VALUES(?,?,?)"
      : table == "downloads"
          ? "INSERT INTO downloads(url,path,state,percent,created_at) "
            "VALUES(?,?,?,?,?)"
          : "INSERT INTO logs(level,message,created_at) VALUES(?,?,?)";
  sqlite3_stmt *statement = nullptr;
  sqlite3_prepare_v2(db_, sql.c_str(), -1, &statement, nullptr);
  for (size_t i = 0; i < list->GetSize(); ++i) {
    auto item = list->GetDictionary(i);
    if (!item) {
      continue;
    }
    sqlite3_reset(statement);
    if (table == "bookmarks") {
      BindText(statement, 1, item->GetString("title").ToString());
      BindText(statement, 2, item->GetString("url").ToString());
      BindText(statement, 3, item->GetString("faviconUrl").ToString());
      BindText(statement, 4, item->GetString("createdAt").ToString());
    } else if (table == "history") {
      BindText(statement, 1, item->GetString("title").ToString());
      BindText(statement, 2, item->GetString("url").ToString());
      BindText(statement, 3, item->GetString("createdAt").ToString());
    } else if (table == "downloads") {
      BindText(statement, 1, item->GetString("url").ToString());
      BindText(statement, 2, item->GetString("path").ToString());
      BindText(statement, 3, item->GetString("state").ToString());
      sqlite3_bind_int(statement, 4, item->GetInt("percent"));
      BindText(statement, 5, item->GetString("createdAt").ToString());
    } else {
      BindText(statement, 1, item->GetString("level").ToString());
      BindText(statement, 2, item->GetString("message").ToString());
      BindText(statement, 3, item->GetString("createdAt").ToString());
    }
    sqlite3_step(statement);
  }
  sqlite3_finalize(statement);
}

void BrowserDataStore::RefreshCaches() {
  RefreshSettings();
  RefreshList("history", history_, kMaxHistoryItems);
  RefreshList("bookmarks", bookmarks_, 500);
  RefreshList("downloads", downloads_, kMaxDownloadItems);
  RefreshList("site_permissions", permissions_, 500);
  RefreshList("logs", logs_, kMaxLogItems);
}

void BrowserDataStore::RefreshSettings() {
  settings_ = CefDictionaryValue::Create();
  sqlite3_stmt *statement = nullptr;
  sqlite3_prepare_v2(db_, "SELECT key,value FROM settings", -1, &statement,
                     nullptr);
  while (sqlite3_step(statement) == SQLITE_ROW) {
    settings_->SetString(ColumnText(statement, 0), ColumnText(statement, 1));
  }
  sqlite3_finalize(statement);
}

void BrowserDataStore::RefreshList(const std::string &table,
                                   CefRefPtr<CefListValue> target,
                                   size_t limit) {
  target->Clear();
  const std::string sql =
      table == "bookmarks"
          ? "SELECT title,url,favicon_url,created_at,NULL,NULL,NULL FROM "
            "bookmarks ORDER BY id DESC LIMIT ?"
      : table == "history" ? "SELECT title,url,NULL,created_at,NULL,NULL,NULL "
                             "FROM history ORDER BY id DESC LIMIT ?"
      : table == "downloads"
          ? "SELECT "
            "NULL,url,NULL,COALESCE(updated_at,created_at),path,state,percent "
            "FROM downloads ORDER BY COALESCE(updated_at,created_at) DESC,id "
            "DESC LIMIT ?"
      : table == "site_permissions"
          ? "SELECT origin,permission,NULL,updated_at,NULL,value,NULL FROM "
            "site_permissions ORDER BY updated_at DESC LIMIT ?"
          : "SELECT NULL,NULL,NULL,created_at,NULL,level,message FROM logs "
            "ORDER BY id DESC LIMIT ?";
  sqlite3_stmt *statement = nullptr;
  sqlite3_prepare_v2(db_, sql.c_str(), -1, &statement, nullptr);
  sqlite3_bind_int(statement, 1, static_cast<int>(limit));
  size_t index = 0;
  while (sqlite3_step(statement) == SQLITE_ROW) {
    auto item = CefDictionaryValue::Create();
    item->SetString("title", ColumnText(statement, 0));
    item->SetString("url", ColumnText(statement, 1));
    item->SetString("faviconUrl", ColumnText(statement, 2));
    item->SetString("createdAt", ColumnText(statement, 3));
    item->SetString("path", ColumnText(statement, 4));
    item->SetString("state", ColumnText(statement, 5));
    item->SetString("permission", ColumnText(statement, 1));
    item->SetString("value", ColumnText(statement, 5));
    item->SetInt("percent", sqlite3_column_type(statement, 6) == SQLITE_NULL
                                ? 0
                                : sqlite3_column_int(statement, 6));
    item->SetString("level", ColumnText(statement, 5));
    item->SetString("message", ColumnText(statement, 6));
    target->SetDictionary(index++, item);
  }
  sqlite3_finalize(statement);
}

std::string BrowserDataStore::NowIsoString() const {
  const auto now = std::chrono::system_clock::now();
  const auto time = std::chrono::system_clock::to_time_t(now);
  std::tm tm{};
  localtime_r(&time, &tm);
  std::ostringstream out;
  out << std::put_time(&tm, "%Y-%m-%dT%H:%M:%S%z");
  return out.str();
}

}  // namespace fubuki
