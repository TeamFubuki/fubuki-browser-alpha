#pragma once

#include <cstdint>
#include <functional>
#include <string>
#include <unordered_map>
#include <utility>
#include <vector>

namespace fubuki {

// CEF's public bit values; the adapter asserts these against its CEF headers.
inline bool PermissionNames(uint32_t requested, std::vector<std::string>& names) {
  constexpr uint32_t known =
      (1u << 1) | (1u << 2) | (1u << 8) | (1u << 12) | (1u << 15) | (1u << 16) | (1u << 17);
  names.clear();
  if (!requested || (requested & ~known))
    return false;
  if (requested & ((1u << 1) | (1u << 2)))
    names.emplace_back("camera");
  if (requested & (1u << 12))
    names.emplace_back("microphone");
  if (requested & (1u << 8))
    names.emplace_back("geolocation");
  if (requested & (1u << 15))
    names.emplace_back("notifications");
  if (requested & (1u << 17))
    names.emplace_back("pointerLock");
  if (requested & (1u << 16))
    names.emplace_back("keyboardLock");
  return true;
}

inline bool MediaPermissionNames(uint32_t requested, std::vector<std::string>& names) {
  names.clear();
  if (!requested || (requested & ~3u))
    return false;
  if (requested & 1u)
    names.emplace_back("microphone");
  if (requested & 2u)
    names.emplace_back("camera");
  return true;
}

// Erase before invoking CEF: Continue/Cancel may synchronously reenter us.
class PermissionCallbacks {
 public:
  using Completion = std::function<void(const std::string&)>;
  uint64_t Add(const std::string& id, Completion completion) {
    const uint64_t token = ++nextToken_;
    return pending_.emplace(id, Entry{token, std::move(completion)}).second ? token : 0;
  }
  bool Resolve(const std::string& id, const std::string& decision, uint64_t token = 0) {
    auto it = pending_.find(id);
    if (it == pending_.end() || (token && token != it->second.token))
      return false;
    auto completion = std::move(it->second.completion);
    pending_.erase(it);
    completion(decision);
    return true;
  }
  bool Remove(const std::string& id) {
    return pending_.erase(id) != 0;
  }
  bool Contains(const std::string& id) const {
    return pending_.contains(id);
  }
  void Cancel() {
    auto pending = std::move(pending_);
    pending_.clear();
    for (auto& [id, entry] : pending)
      entry.completion("ask");
  }

 private:
  struct Entry {
    uint64_t token;
    Completion completion;
  };
  std::unordered_map<std::string, Entry> pending_;
  uint64_t nextToken_ = 0;
};

}  // namespace fubuki
