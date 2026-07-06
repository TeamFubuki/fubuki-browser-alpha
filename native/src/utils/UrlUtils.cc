#include "utils/UrlUtils.h"

#include <algorithm>
#include <cctype>
#include <optional>
#include <sstream>
#include <string_view>
#include <vector>

namespace fubuki {

namespace {

std::string Trim(std::string value) {
  auto notSpace = [](unsigned char c) { return !std::isspace(c); };
  value.erase(value.begin(),
              std::find_if(value.begin(), value.end(), notSpace));
  value.erase(std::find_if(value.rbegin(), value.rend(), notSpace).base(),
              value.end());
  return value;
}

bool ContainsWhitespace(const std::string& value) {
  return std::any_of(value.begin(), value.end(), [](unsigned char c) { return std::isspace(c); });
}

bool ContainsNonAscii(const std::string& value) {
  return std::any_of(value.begin(), value.end(), [](unsigned char c) { return c >= 0x80; });
}

bool HasScheme(const std::string& value) {
  if (value.empty() || !std::isalpha(static_cast<unsigned char>(value.front()))) {
    return false;
  }

  for (size_t i = 1; i < value.size(); ++i) {
    const unsigned char c = static_cast<unsigned char>(value[i]);
    if (c == ':') {
      return true;
    }
    if (!(std::isalnum(c) || c == '+' || c == '.' || c == '-')) {
      return false;
    }
  }
  return false;
}

std::string_view StripPathQueryFragment(std::string_view value) {
  const size_t end = value.find_first_of("/?#");
  return end == std::string_view::npos ? value : value.substr(0, end);
}

bool IsAllDigits(std::string_view value) {
  return !value.empty() &&
         std::all_of(value.begin(), value.end(), [](unsigned char c) { return std::isdigit(c); });
}

std::optional<int> ParsePort(std::string_view value) {
  if (!IsAllDigits(value)) {
    return std::nullopt;
  }
  int port = 0;
  for (const char c : value) {
    port = port * 10 + (c - '0');
    if (port > 65535) {
      return std::nullopt;
    }
  }
  return port;
}

std::string_view StripPort(std::string_view authority) {
  if (authority.empty() || authority.front() == '[') {
    return authority;
  }
  const size_t colon = authority.rfind(':');
  if (colon == std::string_view::npos) {
    return authority;
  }
  return ParsePort(authority.substr(colon + 1)).has_value() ? authority.substr(0, colon) : authority;
}

std::string_view HostPart(const std::string& value) {
  return StripPort(StripPathQueryFragment(value));
}

bool IsAsciiAlphaNumHyphen(std::string_view value) {
  return !value.empty() && value.front() != '-' && value.back() != '-' &&
         std::all_of(value.begin(), value.end(), [](unsigned char c) {
           return std::isalnum(c) || c == '-';
         });
}

std::vector<std::string_view> Split(std::string_view value, char delimiter) {
  std::vector<std::string_view> parts;
  size_t start = 0;
  while (start <= value.size()) {
    const size_t end = value.find(delimiter, start);
    if (end == std::string_view::npos) {
      parts.push_back(value.substr(start));
      break;
    }
    parts.push_back(value.substr(start, end - start));
    start = end + 1;
  }
  return parts;
}

bool LooksLikeIpv4Address(std::string_view host) {
  const auto parts = Split(host, '.');
  if (parts.size() != 4) {
    return false;
  }
  for (const auto part : parts) {
    if (!IsAllDigits(part) || part.size() > 3) {
      return false;
    }
    int octet = 0;
    for (const char c : part) {
      octet = octet * 10 + (c - '0');
    }
    if (octet > 255) {
      return false;
    }
  }
  return true;
}

int Ipv4Octet(std::string_view host, size_t index) {
  const auto parts = Split(host, '.');
  int octet = 0;
  for (const char c : parts[index]) {
    octet = octet * 10 + (c - '0');
  }
  return octet;
}

bool LooksLikePrivateOrLoopbackIpv4(std::string_view host) {
  if (!LooksLikeIpv4Address(host)) {
    return false;
  }
  const int first = Ipv4Octet(host, 0);
  const int second = Ipv4Octet(host, 1);
  return first == 10 || first == 127 || (first == 172 && second >= 16 && second <= 31) ||
         (first == 192 && second == 168) || (first == 169 && second == 254);
}

bool LooksLikeBracketedIpv6(const std::string& value) {
  const std::string_view authority = StripPathQueryFragment(value);
  if (authority.empty() || authority.front() != '[') {
    return false;
  }
  const size_t close = authority.find(']');
  if (close == std::string_view::npos || close == 1) {
    return false;
  }
  const std::string_view address = authority.substr(1, close - 1);
  if (address.find(':') == std::string_view::npos) {
    return false;
  }
  const bool addressCharsOk = std::all_of(address.begin(), address.end(), [](unsigned char c) {
    return std::isxdigit(c) || c == ':' || c == '.';
  });
  if (!addressCharsOk) {
    return false;
  }
  if (close + 1 == authority.size()) {
    return true;
  }
  return authority[close + 1] == ':' && ParsePort(authority.substr(close + 2)).has_value();
}

bool LooksLikeLocalhost(std::string_view host) {
  if (host.size() != 9) {
    return false;
  }
  const std::string localhost = "localhost";
  for (size_t i = 0; i < localhost.size(); ++i) {
    if (std::tolower(static_cast<unsigned char>(host[i])) != localhost[i]) {
      return false;
    }
  }
  return true;
}

bool LooksLikeDomain(std::string_view host, bool containsNonAscii) {
  if (host.find('.') == std::string_view::npos) {
    return false;
  }
  if (containsNonAscii) {
    return true;
  }
  const auto labels = Split(host, '.');
  if (labels.size() < 2) {
    return false;
  }
  return std::all_of(labels.begin(), labels.end(), IsAsciiAlphaNumHyphen);
}

bool LooksLikeLocalHostnameWithPort(const std::string& value) {
  const std::string_view authority = StripPathQueryFragment(value);
  if (authority.empty() || authority.front() == '[') {
    return false;
  }
  const size_t colon = authority.rfind(':');
  if (colon == std::string_view::npos || !ParsePort(authority.substr(colon + 1)).has_value()) {
    return false;
  }
  const std::string_view host = authority.substr(0, colon);
  return host.find('.') == std::string_view::npos && IsAsciiAlphaNumHyphen(host);
}

bool LooksLikeHostWithPort(const std::string& value) {
  const std::string_view authority = StripPathQueryFragment(value);
  if (authority.empty() || authority.front() == '[') {
    return false;
  }
  const size_t colon = authority.rfind(':');
  if (colon == std::string_view::npos || !ParsePort(authority.substr(colon + 1)).has_value()) {
    return false;
  }
  const std::string_view host = authority.substr(0, colon);
  return LooksLikeLocalhost(host) || LooksLikeIpv4Address(host) ||
         LooksLikeDomain(host, ContainsNonAscii(value)) ||
         (host.find('.') == std::string_view::npos && IsAsciiAlphaNumHyphen(host));
}

bool LooksLikeDotLocal(std::string_view host) {
  static constexpr std::string_view suffix = ".local";
  if (host.size() <= suffix.size()) {
    return false;
  }
  const std::string_view ending = host.substr(host.size() - suffix.size());
  for (size_t i = 0; i < suffix.size(); ++i) {
    if (std::tolower(static_cast<unsigned char>(ending[i])) != suffix[i]) {
      return false;
    }
  }
  return true;
}

bool LooksLikeUrlInput(const std::string& value) {
  if (ContainsWhitespace(value)) {
    return false;
  }
  if (LooksLikeHostWithPort(value) || HasScheme(value) || LooksLikeBracketedIpv6(value)) {
    return true;
  }
  const std::string_view host = HostPart(value);
  const bool nonAscii = ContainsNonAscii(value);
  return LooksLikeLocalhost(host) || LooksLikeIpv4Address(host) || LooksLikeDomain(host, nonAscii);
}

bool ShouldPreferHttp(const std::string& value) {
  if (LooksLikeBracketedIpv6(value) || LooksLikeLocalHostnameWithPort(value)) {
    return true;
  }
  const std::string_view host = HostPart(value);
  return LooksLikeLocalhost(host) || LooksLikePrivateOrLoopbackIpv4(host) || LooksLikeDotLocal(host);
}

std::string ReplaceAll(std::string value, const std::string &needle,
                       const std::string &replacement) {
  size_t position = 0;
  while ((position = value.find(needle, position)) != std::string::npos) {
    value.replace(position, needle.size(), replacement);
    position += replacement.size();
  }
  return value;
}

std::string SearchUrlFor(const std::string &engine,
                         const std::string &customSearchUrl,
                         const std::string &query) {
  const std::string escaped = EscapeQuery(query);
  if (engine == "custom" && !customSearchUrl.empty()) {
    if (customSearchUrl.find("{query}") != std::string::npos) {
      return ReplaceAll(customSearchUrl, "{query}", escaped);
    }
    if (customSearchUrl.find("%s") != std::string::npos) {
      return ReplaceAll(customSearchUrl, "%s", escaped);
    }
    return customSearchUrl +
           (customSearchUrl.find('?') == std::string::npos ? "?q=" : "&q=") +
           escaped;
  }
  if (engine == "google") {
    return "https://www.google.com/search?q=" + escaped;
  }
  if (engine == "bing") {
    return "https://www.bing.com/search?q=" + escaped;
  }
  return "https://duckduckgo.com/?q=" + escaped;
}

}  // namespace

std::string EscapeQuery(const std::string &input) {
  std::ostringstream out;
  for (unsigned char c : input) {
    if (std::isalnum(c) || c == '-' || c == '_' || c == '.' || c == '~') {
      out << c;
    } else if (c == ' ') {
      out << '+';
    } else {
      static constexpr char hex[] = "0123456789ABCDEF";
      out << '%' << hex[c >> 4] << hex[c & 15];
    }
  }
  return out.str();
}

std::string NormalizeNavigationInput(const std::string &input) {
  return NormalizeNavigationInput(input, "google", "");
}

std::string NormalizeNavigationInput(const std::string &input,
                                     const std::string &searchEngine) {
  return NormalizeNavigationInput(input, searchEngine, "");
}

std::string NormalizeNavigationInput(const std::string &input,
                                     const std::string &searchEngine,
                                     const std::string &customSearchUrl) {
  const std::string value = Trim(input);
  if (value.empty()) {
    return "fubuki://newtab/";
  }
  if (LooksLikeHostWithPort(value)) {
    return std::string(ShouldPreferHttp(value) ? "http://" : "https://") + value;
  }
  if (HasScheme(value)) {
    return value;
  }
  if (LooksLikeUrlInput(value)) {
    return std::string(ShouldPreferHttp(value) ? "http://" : "https://") + value;
  }
  return SearchUrlFor(searchEngine, customSearchUrl, value);
}

}  // namespace fubuki
