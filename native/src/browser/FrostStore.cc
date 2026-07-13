#include "browser/FrostStore.h"

#include <stdexcept>

#include "frost_ffi.h"
#include "include/cef_parser.h"
#include "include/cef_values.h"
#include "utils/JsonUtils.h"

namespace fubuki {

namespace {

std::string TakeEngineString(char *value) {
  if (!value) {
    return "";
  }
  std::string result(value);
  frost_engine_string_free(value);
  return result;
}

}  // namespace

FrostStore::FrostStore(std::filesystem::path profilePath, void *engineHandle)
    : profilePath_(std::move(profilePath)), engine_(engineHandle) {
  if (!engine_) {
    throw std::runtime_error("FrostEngine store is unavailable");
  }
}

std::string FrostStore::GetSetting(const std::string &key) const {
  CefRefPtr<CefValue> value = CefParseJSON(
      ExecRequestResult("settings.get", "{\"key\":" + JsonEscape(key) + "}"),
      JSON_PARSER_RFC);
  return value && value->GetType() == VTYPE_STRING
             ? value->GetString().ToString()
             : "";
}

bool FrostStore::SetSetting(const std::string &key, const std::string &value) {
  return ExecRequest("settings.set", "{\"key\":" + JsonEscape(key) +
                                         ",\"value\":" + JsonEscape(value) + "}");
}

std::string FrostStore::GetAllSettings() const {
  const std::string snapshot = ExecRequestResult("app.snapshot", "{}");
  CefRefPtr<CefValue> parsed = CefParseJSON(snapshot, JSON_PARSER_RFC);
  if (!parsed || parsed->GetType() != VTYPE_DICTIONARY ||
      parsed->GetDictionary()->GetType("settings") != VTYPE_DICTIONARY) {
    return "{}";
  }
  auto value = CefValue::Create();
  value->SetDictionary(parsed->GetDictionary()->GetDictionary("settings"));
  return CefWriteJSON(value, JSON_WRITER_DEFAULT).ToString();
}

bool FrostStore::AddLog(const std::string &level, const std::string &message) {
  return ExecRequest("logs.add", "{\"level\":" + JsonEscape(level) +
                                    ",\"message\":" + JsonEscape(message) + "}");
}

std::string FrostStore::GetLogs(size_t limit) const {
  return ExecRequestResult("logs.list",
                           "{\"limit\":" + std::to_string(limit) + "}");
}

bool FrostStore::ClearLogs() {
  return ExecRequest("logs.clear", "{}");
}

bool FrostStore::ExecRequest(const std::string &method,
                             const std::string &paramsJson) {
  const std::string result = ExecRequestResult(method, paramsJson);
  return result == "true";
}

std::string FrostStore::ExecRequestResult(const std::string &method,
                                          const std::string &paramsJson) const {
  std::string request = "{\"version\":0,\"method\":\"" + method +
                        "\",\"params\":" + paramsJson + "}";
  std::string response =
      TakeEngineString(frost_engine_process_json(engine_, request.c_str()));
  CefRefPtr<CefValue> parsed = CefParseJSON(response, JSON_PARSER_RFC);
  if (!parsed || parsed->GetType() != VTYPE_DICTIONARY) {
    return "";
  }
  CefRefPtr<CefDictionaryValue> envelope = parsed->GetDictionary();
  if (!envelope->GetBool("ok") || !envelope->HasKey("result")) {
    return "";
  }
  CefRefPtr<CefValue> result = envelope->GetValue("result");
  return result ? CefWriteJSON(result, JSON_WRITER_DEFAULT).ToString() : "";
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

bool FrostStore::RemoveHistory(const std::string &url) {
  std::string params = "{\"url\":" + JsonEscape(url) + "}";
  return ExecRequest("history.remove", params);
}

bool FrostStore::RemoveDownload(const std::string &url,
                                const std::string &path) {
  std::string params = "{\"url\":" + JsonEscape(url) + ",\"path\":" + JsonEscape(path) + "}";
  return ExecRequest("downloads.remove", params);
}

bool FrostStore::HasDownloadPath(const std::string &path) const {
  if (path.empty()) {
    return false;
  }
  CefRefPtr<CefValue> parsed =
      CefParseJSON(ExecRequestResult("downloads.list", "{}"), JSON_PARSER_RFC);
  if (!parsed || parsed->GetType() != VTYPE_LIST) {
    return false;
  }
  CefRefPtr<CefListValue> downloads = parsed->GetList();
  std::error_code error;
  const std::filesystem::path requested = std::filesystem::canonical(path, error);
  if (error) {
    return false;
  }
  for (size_t index = 0; index < downloads->GetSize(); ++index) {
    CefRefPtr<CefDictionaryValue> download = downloads->GetDictionary(index);
    if (!download || download->GetString("state") != "completed") {
      continue;
    }
    const std::filesystem::path registered = std::filesystem::canonical(
        download->GetString("path").ToString(), error);
    if (!error && registered == requested) {
      return true;
    }
  }
  return false;
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
