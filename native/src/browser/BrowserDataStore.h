#pragma once

#include <filesystem>
#include <string>

#include "include/cef_values.h"

namespace fubuki {

class BrowserDataStore {
 public:
  explicit BrowserDataStore(std::filesystem::path profilePath);

  void Load();
  void AddHistory(const std::string& title, const std::string& url);
  bool AddBookmark(const std::string& title, const std::string& url, const std::string& faviconUrl);
  bool RemoveBookmark(const std::string& url);
  void AddDownload(const std::string& url, const std::string& path, const std::string& state);
  void UpdateDownload(const std::string& url, const std::string& path, const std::string& state, int percent);
  void Log(const std::string& level, const std::string& message);
  void SetSetting(const std::string& key, const std::string& value);

  CefRefPtr<CefListValue> History() const { return history_; }
  CefRefPtr<CefListValue> Bookmarks() const { return bookmarks_; }
  CefRefPtr<CefListValue> Downloads() const { return downloads_; }
  CefRefPtr<CefListValue> Logs() const { return logs_; }
  CefRefPtr<CefDictionaryValue> Settings() const { return settings_; }
  std::string ProfilePath() const { return profilePath_.string(); }

 private:
  CefRefPtr<CefListValue> LoadList(const std::filesystem::path& path);
  CefRefPtr<CefDictionaryValue> LoadDictionary(const std::filesystem::path& path);
  void SaveList(const std::filesystem::path& path, CefRefPtr<CefListValue> list) const;
  void SaveDictionary(const std::filesystem::path& path, CefRefPtr<CefDictionaryValue> dict) const;
  CefRefPtr<CefDictionaryValue> NewRecord() const;
  std::string NowIsoString() const;

  std::filesystem::path profilePath_;
  std::filesystem::path historyPath_;
  std::filesystem::path bookmarksPath_;
  std::filesystem::path downloadsPath_;
  std::filesystem::path settingsPath_;
  std::filesystem::path logsPath_;
  CefRefPtr<CefListValue> history_;
  CefRefPtr<CefListValue> bookmarks_;
  CefRefPtr<CefListValue> downloads_;
  CefRefPtr<CefDictionaryValue> settings_;
  CefRefPtr<CefListValue> logs_;
};

}  // namespace fubuki
