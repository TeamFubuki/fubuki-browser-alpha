#pragma once

#include <filesystem>
#include <string>

#include "include/cef_values.h"

struct sqlite3;

namespace fubuki {

class BrowserDataStore {
 public:
  explicit BrowserDataStore(std::filesystem::path profilePath);
  ~BrowserDataStore();

  void Load();
  void AddHistory(const std::string& title, const std::string& url);
  bool AddBookmark(const std::string& title, const std::string& url, const std::string& faviconUrl);
  bool RemoveBookmark(const std::string& url);
  bool RemoveHistory(const std::string& url);
  bool RemoveDownload(const std::string& url, const std::string& path);
  bool HasDownloadPath(const std::string& path) const;
  bool ClearBookmarks();
  bool ClearHistory();
  bool ClearHistoryRange(const std::string& range);
  bool ClearDownloads();
  bool ClearLogs();
  bool SetPermission(const std::string& origin, const std::string& permission,
                     const std::string& value);
  bool RemovePermission(const std::string& origin, const std::string& permission);
  void AddDownload(const std::string& downloadId, const std::string& url, const std::string& path,
                   const std::string& state);
  void UpdateDownload(const std::string& downloadId, const std::string& url,
                      const std::string& path, const std::string& state, int percent);
  void Log(const std::string& level, const std::string& message);
  void SetSetting(const std::string& key, const std::string& value);
  void ResetSetting(const std::string& key);

  CefRefPtr<CefListValue> History() const {
    return history_;
  }
  CefRefPtr<CefListValue> Bookmarks() const {
    return bookmarks_;
  }
  CefRefPtr<CefListValue> Downloads() const {
    return downloads_;
  }
  CefRefPtr<CefListValue> Permissions() const {
    return permissions_;
  }
  CefRefPtr<CefListValue> Logs() const {
    return logs_;
  }
  CefRefPtr<CefDictionaryValue> Settings() const {
    return settings_;
  }
  std::string ProfilePath() const {
    return profilePath_.string();
  }

 private:
  // BrowserDataStore still owns SQLite access for all persisted browser data.
  // Keep schema-compatible changes here until narrower
  // History/Bookmark/Download stores are split out.
  CefRefPtr<CefDictionaryValue> NewRecord() const;
  void OpenDatabase();
  void Execute(const std::string& sql) const;
  void EnsureSchema();
  void EnsureDefaultSetting(const std::string& key, const std::string& value);
  std::string DefaultSetting(const std::string& key) const;
  int CountRows(const std::string& table) const;
  void MigrateJsonFiles();
  void MigrateSettingsJson(const std::filesystem::path& path);
  void MigrateRecordsJson(const std::filesystem::path& path, const std::string& table);
  void RefreshCaches();
  void RefreshSettings();
  void RefreshList(const std::string& table, CefRefPtr<CefListValue> target, size_t limit);
  std::string NowIsoString() const;

  std::filesystem::path profilePath_;
  std::filesystem::path databasePath_;
  sqlite3* db_ = nullptr;
  CefRefPtr<CefListValue> history_;
  CefRefPtr<CefListValue> bookmarks_;
  CefRefPtr<CefListValue> downloads_;
  CefRefPtr<CefListValue> permissions_;
  CefRefPtr<CefDictionaryValue> settings_;
  CefRefPtr<CefListValue> logs_;
};

}  // namespace fubuki
