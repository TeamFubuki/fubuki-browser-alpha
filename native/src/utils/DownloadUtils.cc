#include "utils/DownloadUtils.h"

#include <cstdint>
#include <system_error>

namespace fubuki {
namespace {

constexpr size_t kMaxFilenameBytes = 200;
constexpr uint32_t kMaxCollisionAttempts = 10'000;

bool IsAsciiWhitespace(unsigned char value) {
  return value == ' ' || value == '\t' || value == '\n' || value == '\r' || value == '\f' ||
         value == '\v';
}

bool IsUnsafeFilenameByte(unsigned char value) {
  if (value < 0x20 || value == 0x7f) {
    return true;
  }
  switch (value) {
    case '/':
    case '\\':
    case '<':
    case '>':
    case ':':
    case '"':
    case '|':
    case '?':
    case '*':
      return true;
    default:
      return false;
  }
}

void TrimAsciiWhitespace(std::string& value) {
  size_t first = 0;
  while (first < value.size() && IsAsciiWhitespace(static_cast<unsigned char>(value[first]))) {
    ++first;
  }
  size_t last = value.size();
  while (last > first && IsAsciiWhitespace(static_cast<unsigned char>(value[last - 1]))) {
    --last;
  }
  value = value.substr(first, last - first);
}

void StripTrailingDotsAndSpaces(std::string& value) {
  while (!value.empty() && (value.back() == '.' || value.back() == ' ')) {
    value.pop_back();
  }
}

void TruncateUtf8(std::string& value) {
  if (value.size() <= kMaxFilenameBytes) {
    return;
  }
  size_t end = kMaxFilenameBytes;
  while (end > 0 && (static_cast<unsigned char>(value[end]) & 0xc0) == 0x80) {
    --end;
  }
  value.resize(end);
}

enum class CandidateState { kAvailable, kOccupied, kError };

CandidateState CheckCandidate(const std::filesystem::path& candidate) {
  std::error_code error;
  const auto status = std::filesystem::symlink_status(candidate, error);
  if (!error) {
    return status.type() == std::filesystem::file_type::not_found ? CandidateState::kAvailable
                                                                  : CandidateState::kOccupied;
  }
  if (error == std::errc::no_such_file_or_directory) {
    return CandidateState::kAvailable;
  }
  return CandidateState::kError;
}

}  // namespace

std::string SanitizeDownloadFilename(std::string_view suggestedName) {
  const size_t separator = suggestedName.find_last_of("/\\");
  std::string filename(
      suggestedName.substr(separator == std::string_view::npos ? 0 : separator + 1));
  TrimAsciiWhitespace(filename);

  for (char& value : filename) {
    if (IsUnsafeFilenameByte(static_cast<unsigned char>(value))) {
      value = '_';
    }
  }

  StripTrailingDotsAndSpaces(filename);
  TruncateUtf8(filename);
  StripTrailingDotsAndSpaces(filename);
  return filename.empty() || filename == "." || filename == ".." ? "download" : filename;
}

std::filesystem::path UniqueDownloadPath(const std::filesystem::path& directory,
                                         std::string_view suggestedName) {
  const std::filesystem::path filename(SanitizeDownloadFilename(suggestedName));
  std::filesystem::path candidate = directory / filename;
  CandidateState state = CheckCandidate(candidate);
  if (state == CandidateState::kAvailable) {
    return candidate;
  }
  if (state == CandidateState::kError) {
    return {};
  }

  const std::string stem = filename.stem().string();
  const std::string extension = filename.extension().string();
  for (uint32_t index = 1; index <= kMaxCollisionAttempts; ++index) {
    candidate = directory / (stem + " (" + std::to_string(index) + ")" + extension);
    state = CheckCandidate(candidate);
    if (state == CandidateState::kAvailable) {
      return candidate;
    }
    if (state == CandidateState::kError) {
      return {};
    }
  }
  return {};
}

}  // namespace fubuki
