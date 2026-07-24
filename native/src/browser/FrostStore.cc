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
  if (handle_) {
    frost_store_free(handle_);
    handle_ = nullptr;
  }
}

std::string FrostStore::GetSetting(const std::string &key) const {
  if (!handle_) {
    return "";
  }
  return CStrOrEmpty(frost_store_get_setting(handle_, key.c_str()));
}

bool FrostStore::SetSetting(const std::string &key, const std::string &value) {
  if (!handle_) {
    return false;
  }
  const std::string params = "{\"key\":" + JsonEscape(key) +
                             ",\"value\":" + JsonEscape(value) + "}";
  return ExecRequest("settings.set", params);
}

std::string FrostStore::GetAllSettings() const {
  if (!handle_) {
    return "{}";
  }
  return CStrOrEmpty(frost_store_get_all_settings(handle_));
}

std::string FrostStore::GetSession() const {
  if (!handle_) {
    return "";
  }
  return CStrOrEmpty(frost_store_get_session(handle_));
}

bool FrostStore::SetSession(const std::string &json) {
  return handle_ && frost_store_set_session(handle_, json.c_str());
}

bool FrostStore::AddLog(const std::string &level, const std::string &message) {
  if (!handle_) {
    return false;
  }
  return frost_store_add_log(handle_, level.c_str(), message.c_str());
}

std::string FrostStore::GetLogs(size_t limit) const {
  if (!handle_) {
    return "[]";
  }
  return CStrOrEmpty(frost_store_get_logs(handle_, limit));
}

bool FrostStore::ClearLogs() {
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
  // The host no longer owns download metadata; treat any non-empty path as
  // valid so file-open/reveal still works for completed downloads.
  return !path.empty();
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
