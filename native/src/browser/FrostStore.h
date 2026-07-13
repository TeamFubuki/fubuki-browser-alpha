#pragma once

#include <filesystem>
#include <string>

namespace fubuki {

// Thin C++ wrapper around the engine-owned SQLite store exposed by `frost-ffi`.
//
// The native host no longer owns browser data. All persistence (settings,
// logs, permissions, history, bookmarks, downloads, session) lives in
// FrostEngine's `frost-store`, accessed here through the FFI boundary. This
// keeps the host as a pure I/O layer per the architecture plan.
class FrostStore {
public:
  // Opens the engine store at `profilePath / "frost-engine.sqlite3"`.
  // `engineHandle` is the FrostEngine instance used to delegate browser-data
  // mutations (bookmarks, history, downloads, permissions) through the
  // protocol layer. May be null if the engine is unavailable.
  FrostStore(std::filesystem::path profilePath, void *engineHandle);
  ~FrostStore() = default;

  FrostStore(const FrostStore &) = delete;
  FrostStore &operator=(const FrostStore &) = delete;

  // --- Settings -----------------------------------------------------------
  std::string GetSetting(const std::string &key) const;
  bool SetSetting(const std::string &key, const std::string &value);
  // Returns all known settings as a JSON object string.
  std::string GetAllSettings() const;

  // --- Logs -----------------------------------------------------------------
  bool AddLog(const std::string &level, const std::string &message);
  std::string GetLogs(size_t limit) const;
  bool ClearLogs();

  // --- Browser data (delegated to FrostEngine services via protocol) -----
  // These exist so the host can perform user-initiated data mutations through
  // the engine rather than owning the data itself.
  bool AddBookmark(const std::string &title, const std::string &url,
                   const std::string &faviconUrl);
  bool RemoveBookmark(const std::string &url);
  bool RemoveHistory(const std::string &url);
  bool RemoveDownload(const std::string &url, const std::string &path);
  bool HasDownloadPath(const std::string &path) const;
  bool SetPermission(const std::string &origin, const std::string &permission,
                     const std::string &value);
  bool ClearBookmarks();
  bool ClearHistory();
  bool ClearDownloads();
  bool ClearHistoryRange(const std::string &range);
  bool ResetSetting(const std::string &key);

  std::string ProfilePath() const { return profilePath_.string(); }

private:
  // Issues a Frost Protocol request through the engine and returns the
  // "ok" boolean from the response.
  bool ExecRequest(const std::string &method, const std::string &paramsJson);
  std::string ExecRequestResult(const std::string &method,
                                const std::string &paramsJson) const;

  std::filesystem::path profilePath_;
  void *engine_ = nullptr;
};

}  // namespace fubuki
