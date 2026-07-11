#pragma once

#include <string>

namespace fubuki {

inline std::string ResolveHostCommandWindowId(const std::string& explicitWindowId,
                                              const std::string& tabOwnerWindowId) {
  return explicitWindowId.empty() ? tabOwnerWindowId : explicitWindowId;
}

}  // namespace fubuki
