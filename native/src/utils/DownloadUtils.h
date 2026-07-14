#pragma once

#include <filesystem>
#include <string>
#include <string_view>

namespace fubuki {

// Treats the remote-supplied value strictly as a filename. Both POSIX and
// Windows separators are recognized so a foreign path cannot escape the
// configured download directory.
std::string SanitizeDownloadFilename(std::string_view suggestedName);

// Returns an unused path below directory, adding " (n)" before the extension
// when necessary. Returns an empty path when availability cannot be checked.
std::filesystem::path UniqueDownloadPath(const std::filesystem::path& directory,
                                         std::string_view suggestedName);

}  // namespace fubuki
