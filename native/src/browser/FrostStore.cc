#include "browser/FrostStore.h"

#include <stdexcept>

#include "frost_ffi.h"
#include "utils/JsonUtils.h"

namespace fubuki {

namespace {

std::string CStrOrEmpty(char *value) {
  if (!value) {
    return "";
  }
  std::string result(value);
  frost_store_string_free(value);
  return result;
}

std::optional<std::string> CStrToOptional(char *value) {
  if (!value) {
    return std::nullopt;
  }
  std::string result(value);
  frost_store_string_free(value);
  return result;
}

}  // namespace

FrostStore::FrostStore(std::filesystem::path profilePath, void *engineHandle)
    : profilePath_(std::move(profilePath)), engine_(engineHandle) {
  // Ensure the profile directory exists before opening the database.
  std::error_code ec;
  std::filesystem::create_directories(profilePath_, ec);
  const std::filesystem::path dbPath = profilePath_ / "frost-engine.sqlite3";
  handle_ = frost_store_open(dbPath.string().c_str());
  if (!handle_) {
    throw std::runtime_error("Failed to open engine store at " +
                             dbPath.string());
  }
}

FrostStore::~FrostStore() {
  std::lock_guard lock(handleMutex_);
  if (handle_) {
    frost_store_free(handle_);
    handle_ = nullptr;
  }
}

std::string FrostStore::GetSetting(const std::string &key) const {
  std::lock_guard lock(handleMutex_);
  if (!handle_) {
    return "";
  }
  return CStrOrEmpty(frost_store_get_setting(handle_, key.c_str()));
}

bool FrostStore::SetSetting(const std::string &key, const std::string &value) {
  // Session restoration is still a host concern and is not part of the public
  // settings protocol. All user-facing settings go through FrostEngine so the
  // core remains the single writer and emits the corresponding diff event.
  if (key != "sessionJson") {
    const std::string params = "{\"key\":" + JsonEscape(key) +
                               ",\"value\":" + JsonEscape(value) + "}";
    return ExecRequest("settings.set", params);
  }
  std::lock_guard lock(handleMutex_);
  if (!handle_) {
    return false;
  }
  return frost_store_set_setting(handle_, key.c_str(), value.c_str());
}

std::string FrostStore::GetAllSettings() const {
  return TryGetAllSettings().value_or("{}");
}

std::optional<std::string> FrostStore::TryGetAllSettings() const {
  std::lock_guard lock(handleMutex_);
  return handle_ ? CStrToOptional(frost_store_get_all_settings(handle_))
                 : std::nullopt;
}

bool FrostStore::AddLog(const std::string &level, const std::string &message) {
  std::lock_guard lock(handleMutex_);
  if (!handle_) {
    return false;
  }
  return frost_store_add_log(handle_, level.c_str(), message.c_str());
}

std::string FrostStore::GetLogs(size_t limit) const {
  return TryGetLogs(limit).value_or("[]");
}

std::optional<std::string> FrostStore::TryGetLogs(size_t limit) const {
  std::lock_guard lock(handleMutex_);
  return handle_ ? CStrToOptional(frost_store_get_logs(handle_, limit))
                 : std::nullopt;
}

std::optional<std::string> FrostStore::GetBookmarks(size_t limit) const {
  std::lock_guard lock(handleMutex_);
  return handle_ ? CStrToOptional(frost_store_get_bookmarks(handle_, limit))
                 : std::nullopt;
}

std::optional<std::string> FrostStore::GetHistory(size_t limit) const {
  std::lock_guard lock(handleMutex_);
  return handle_ ? CStrToOptional(frost_store_get_history(handle_, limit))
                 : std::nullopt;
}

std::optional<std::string> FrostStore::GetDownloads(size_t limit) const {
  std::lock_guard lock(handleMutex_);
  return handle_ ? CStrToOptional(frost_store_get_downloads(handle_, limit))
                 : std::nullopt;
}

bool FrostStore::ClearLogs() {
  std::lock_guard lock(handleMutex_);
  if (!handle_) {
    return false;
  }
  return frost_store_clear_logs(handle_);
}

bool FrostStore::ExecRequest(const std::string &method,
                             const std::string &paramsJson) {
  if (!engine_) {
    return false;
  }
  std::string request = "{\"version\":0,\"method\":\"" + method +
                        "\",\"params\":" + paramsJson + "}";
  std::string response =
      CStrOrEmpty(frost_engine_process_json(engine_, request.c_str()));
  if (response.empty()) {
    return false;
  }
  // Response is {"version":0,"ok":bool,"kind":...,"result":...}
  size_t okPos = response.find("\"ok\":");
  if (okPos == std::string::npos) {
    return false;
  }
  return response.substr(okPos + 5, 4) == "true";
}

bool FrostStore::AddBookmark(const std::string &title, const std::string &url,
                             const std::string &faviconUrl) {
  std::string params = "{\"title\":" + JsonEscape(title) +
                       ",\"url\":" + JsonEscape(url) +
                       ",\"faviconUrl\":" + JsonEscape(faviconUrl) + "}";
  return ExecRequest("bookmarks.save", params);
}

bool FrostStore::RemoveBookmark(const std::string &url) {
  std::string params = "{\"url\":" + JsonEscape(url) + "}";
  return ExecRequest("bookmarks.remove", params);
}

bool FrostStore::AddHistory(const std::string &title, const std::string &url,
                            const std::string &faviconUrl) {
  std::lock_guard lock(handleMutex_);
  if (!handle_) {
    return false;
  }
  return frost_store_add_history(handle_, title.c_str(), url.c_str(),
                                 faviconUrl.c_str());
}

bool FrostStore::RemoveHistory(const std::string &url) {
  std::string params = "{\"url\":" + JsonEscape(url) + "}";
  return ExecRequest("history.remove", params);
}

bool FrostStore::AddDownload(const std::string &url, const std::string &path,
                             const std::string &state) {
  return UpdateDownload(url, path, state, 0);
}

bool FrostStore::UpdateDownload(const std::string &url, const std::string &path,
                                const std::string &state, int percent) {
  std::lock_guard lock(handleMutex_);
  if (!handle_) {
    return false;
  }
  return frost_store_upsert_download(handle_, url.c_str(), path.c_str(),
                                     state.c_str(), percent);
}

bool FrostStore::RemoveDownload(const std::string &url,
                                const std::string &path) {
  std::string params = "{\"url\":" + JsonEscape(url) + ",\"path\":" + JsonEscape(path) + "}";
  return ExecRequest("downloads.remove", params);
}

bool FrostStore::HasDownloadPath(const std::string &path) const {
  std::lock_guard lock(handleMutex_);
  return handle_ && !path.empty() &&
         frost_store_has_download_path(handle_, path.c_str());
}

bool FrostStore::SetPermission(const std::string &origin,
                               const std::string &permission,
                               const std::string &value) {
  std::string params = "{\"origin\":" + JsonEscape(origin) +
                       ",\"permission\":" + JsonEscape(permission) +
                       ",\"value\":" + JsonEscape(value) + "}";
  return ExecRequest("permissions.set", params);
}

bool FrostStore::ClearBookmarks() {
  return ExecRequest("data.clear", "{\"target\":\"bookmarks\"}");
}

bool FrostStore::ClearHistory() {
  return ExecRequest("data.clear", "{\"target\":\"history\"}");
}

bool FrostStore::ClearDownloads() {
  return ExecRequest("data.clear", "{\"target\":\"downloads\"}");
}

bool FrostStore::ClearHistoryRange(const std::string &range) {
  std::string params = "{\"range\":" + JsonEscape(range) + "}";
  return ExecRequest("history.clearRange", params);
}

bool FrostStore::ResetSetting(const std::string &key) {
  return ExecRequest("settings.reset", "{\"key\":" + JsonEscape(key) + "}");
}

}  // namespace fubuki
