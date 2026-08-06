#pragma once

#include <functional>
#include <string>

namespace fubuki {

std::string ResolveRestoredWindowId(
    const std::string &persistedId,
    const std::function<std::string()> &fallback);

std::string NextAvailableWindowId(
    int &nextId,
    const std::function<bool(const std::string &)> &isInUse);

}  // namespace fubuki
