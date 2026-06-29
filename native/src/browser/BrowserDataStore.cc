#include "browser/BrowserDataStore.h"

#include <chrono>
#include <cstdlib>
#include <fstream>
#include <iomanip>
#include <sstream>

#include "include/cef_parser.h"

namespace fubuki {

namespace {

constexpr size_t kMaxHistoryItems = 500;
constexpr size_t kMaxLogItems = 300;

std::string ReadFile(const std::filesystem::path& path) {
  std::ifstream file(path, std::ios::binary);
  if (!file) {
    return "";
  }
  std::ostringstream buffer;
  buffer << file.rdbuf();
  return buffer.str();
}

void WriteFile(const std::filesystem::path& path, const std::string& value) {
  std::filesystem::create_directories(path.parent_path());
  std::ofstream file(path, std::ios::binary | std::ios::trunc);
  file << value;
}

void Prepend(CefRefPtr<CefListValue> list, CefRefPtr<CefDictionaryValue> item, size_t maxItems) {
  auto copy = CefListValue::Create();
  copy->SetDictionary(0, item);
  const size_t limit = std::min(list->GetSize(), maxItems - 1);
  for (size_t i = 0; i < limit; ++i) {
    copy->SetValue(i + 1, list->GetValue(i));
  }
  list->Clear();
  for (size_t i = 0; i < copy->GetSize(); ++i) {
    list->SetValue(i, copy->GetValue(i));
  }
}

}  // namespace

BrowserDataStore::BrowserDataStore(std::filesystem::path profilePath)
    : profilePath_(std::move(profilePath)),
      historyPath_(profilePath_ / "history.json"),
      bookmarksPath_(profilePath_ / "bookmarks.json"),
      downloadsPath_(profilePath_ / "downloads.json"),
      settingsPath_(profilePath_ / "settings.json"),
      logsPath_(profilePath_ / "debug-log.json"),
      history_(CefListValue::Create()),
      bookmarks_(CefListValue::Create()),
      downloads_(CefListValue::Create()),
      settings_(CefDictionaryValue::Create()),
      logs_(CefListValue::Create()) {}

void BrowserDataStore::Load() {
  std::filesystem::create_directories(profilePath_);
  history_ = LoadList(historyPath_);
  bookmarks_ = LoadList(bookmarksPath_);
  downloads_ = LoadList(downloadsPath_);
  logs_ = LoadList(logsPath_);
  settings_ = LoadDictionary(settingsPath_);
  if (!settings_->HasKey("homepage")) {
    settings_->SetString("homepage", "https://example.com");
  }
  if (!settings_->HasKey("searchEngine")) {
    settings_->SetString("searchEngine", "duckduckgo");
  }
  if (!settings_->HasKey("startupBehavior")) {
    settings_->SetString("startupBehavior", "homepage");
  }
  if (!settings_->HasKey("downloadDirectory")) {
    const char* home = std::getenv("HOME");
    settings_->SetString("downloadDirectory", home ? (std::filesystem::path(home) / "Downloads").string() : "/tmp");
  }
  if (!settings_->HasKey("theme")) {
    settings_->SetString("theme", "light");
  }
  SaveDictionary(settingsPath_, settings_);
}

void BrowserDataStore::AddHistory(const std::string& title, const std::string& url) {
  if (url.empty() || url.rfind("fubuki://", 0) == 0 || url.rfind("data:", 0) == 0) {
    return;
  }
  auto item = NewRecord();
  item->SetString("title", title.empty() ? url : title);
  item->SetString("url", url);
  Prepend(history_, item, kMaxHistoryItems);
  SaveList(historyPath_, history_);
}

bool BrowserDataStore::AddBookmark(const std::string& title, const std::string& url, const std::string& faviconUrl) {
  if (url.empty()) {
    return false;
  }
  for (size_t i = 0; i < bookmarks_->GetSize(); ++i) {
    auto item = bookmarks_->GetDictionary(i);
    if (item && item->GetString("url") == url) {
      return true;
    }
  }
  auto item = NewRecord();
  item->SetString("title", title.empty() ? url : title);
  item->SetString("url", url);
  item->SetString("faviconUrl", faviconUrl);
  Prepend(bookmarks_, item, 500);
  SaveList(bookmarksPath_, bookmarks_);
  return true;
}

bool BrowserDataStore::RemoveBookmark(const std::string& url) {
  auto next = CefListValue::Create();
  bool removed = false;
  size_t out = 0;
  for (size_t i = 0; i < bookmarks_->GetSize(); ++i) {
    auto item = bookmarks_->GetDictionary(i);
    if (item && item->GetString("url") == url) {
      removed = true;
      continue;
    }
    next->SetValue(out++, bookmarks_->GetValue(i));
  }
  bookmarks_ = next;
  SaveList(bookmarksPath_, bookmarks_);
  return removed;
}

void BrowserDataStore::AddDownload(const std::string& url, const std::string& path, const std::string& state) {
  UpdateDownload(url, path, state, 0);
}

void BrowserDataStore::UpdateDownload(const std::string& url, const std::string& path, const std::string& state, int percent) {
  auto item = NewRecord();
  item->SetString("url", url);
  item->SetString("path", path);
  item->SetString("state", state);
  item->SetInt("percent", percent);
  Prepend(downloads_, item, 200);
  SaveList(downloadsPath_, downloads_);
}

void BrowserDataStore::Log(const std::string& level, const std::string& message) {
  auto item = NewRecord();
  item->SetString("level", level);
  item->SetString("message", message);
  Prepend(logs_, item, kMaxLogItems);
  SaveList(logsPath_, logs_);
}

void BrowserDataStore::SetSetting(const std::string& key, const std::string& value) {
  settings_->SetString(key, value);
  SaveDictionary(settingsPath_, settings_);
}

CefRefPtr<CefListValue> BrowserDataStore::LoadList(const std::filesystem::path& path) {
  auto parsed = CefParseJSON(ReadFile(path), JSON_PARSER_RFC);
  if (parsed && parsed->GetType() == VTYPE_LIST) {
    return parsed->GetList();
  }
  return CefListValue::Create();
}

CefRefPtr<CefDictionaryValue> BrowserDataStore::LoadDictionary(const std::filesystem::path& path) {
  auto parsed = CefParseJSON(ReadFile(path), JSON_PARSER_RFC);
  if (parsed && parsed->GetType() == VTYPE_DICTIONARY) {
    return parsed->GetDictionary();
  }
  return CefDictionaryValue::Create();
}

void BrowserDataStore::SaveList(const std::filesystem::path& path, CefRefPtr<CefListValue> list) const {
  auto value = CefValue::Create();
  value->SetList(list);
  WriteFile(path, CefWriteJSON(value, JSON_WRITER_PRETTY_PRINT).ToString());
}

void BrowserDataStore::SaveDictionary(const std::filesystem::path& path, CefRefPtr<CefDictionaryValue> dict) const {
  auto value = CefValue::Create();
  value->SetDictionary(dict);
  WriteFile(path, CefWriteJSON(value, JSON_WRITER_PRETTY_PRINT).ToString());
}

CefRefPtr<CefDictionaryValue> BrowserDataStore::NewRecord() const {
  auto item = CefDictionaryValue::Create();
  item->SetString("createdAt", NowIsoString());
  return item;
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
