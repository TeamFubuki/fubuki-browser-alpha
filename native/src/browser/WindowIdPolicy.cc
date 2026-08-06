#include "browser/WindowIdPolicy.h"

namespace fubuki {

std::string ResolveRestoredWindowId(
    const std::string &persistedId,
    const std::function<std::string()> &fallback) {
  return persistedId.empty() ? fallback() : persistedId;
}

std::string NextAvailableWindowId(
    int &nextId,
    const std::function<bool(const std::string &)> &isInUse) {
  std::string candidate;
  do {
    candidate = "window-" + std::to_string(nextId++);
  } while (isInUse(candidate));
  return candidate;
}

}  // namespace fubuki
