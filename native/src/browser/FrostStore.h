#pragma once

#include <filesystem>
#include <mutex>
#include <optional>
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
  ~FrostStore();

  FrostStore(const FrostStore &) = delete;
  FrostStore &operator=(const FrostStore &) = delete;

  // --- Settings -----------------------------------------------------------
  std::string GetSetting(const std::string &key) const;
  bool SetSetting(const std::string &key, const std::string &value);
  // Returns all known settings as a JSON object string.
  std::string GetAllSettings() const;
  std::optional<std::string> TryGetAllSettings() const;

  // --- Logs -----------------------------------------------------------------
  bool AddLog(const std::string &level, const std::string &message);
  std::string GetLogs(size_t limit) const;
  std::optional<std::string> TryGetLogs(size_t limit) const;
  bool ClearLogs();

  // Read-only JSON projections for trusted internal pages. A missing value
  // represents a store/serialization failure, not an empty collection.
  std::optional<std::string> GetBookmarks(size_t limit) const;
  std::optional<std::string> GetHistory(size_t limit) const;
  std::optional<std::string> GetDownloads(size_t limit) const;

  // --- Browser data (delegated to FrostEngine services via protocol) -----
  // These exist so the host can perform user-initiated data mutations through
  // the engine rather than owning the data itself.
  bool AddBookmark(const std::string &title, const std::string &url,
                   const std::string &faviconUrl);
  bool RemoveBookmark(const std::string &url);
  bool AddHistory(const std::string &title, const std::string &url,
                  const std::string &faviconUrl);
  bool RemoveHistory(const std::string &url);
  bool AddDownload(const std::string &url, const std::string &path,
                   const std::string &state);
  bool UpdateDownload(const std::string &url, const std::string &path,
                      const std::string &state, int percent);
  bool RemoveDownload(const std::string &url, const std::string &path);
  // File-system side effects are allowed only for paths present in the
  // engine-owned download ledger.
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

  std::filesystem::path profilePath_;
  mutable std::mutex handleMutex_;
  void *handle_ = nullptr;
  void *engine_ = nullptr;
};

}  // namespace fubuki
