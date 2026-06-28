#include "utils/UrlUtils.h"

#include <algorithm>
#include <cctype>
#include <sstream>

namespace fubuki {

namespace {

std::string Trim(std::string value) {
  auto notSpace = [](unsigned char c) { return !std::isspace(c); };
  value.erase(value.begin(), std::find_if(value.begin(), value.end(), notSpace));
  value.erase(std::find_if(value.rbegin(), value.rend(), notSpace).base(), value.end());
  return value;
}

bool HasScheme(const std::string& value) {
  return value.find("://") != std::string::npos || value.rfind("about:", 0) == 0 || value.rfind("fubuki:", 0) == 0;
}

bool LooksLikeHost(const std::string& value) {
  return value.find(' ') == std::string::npos && value.find('.') != std::string::npos;
}

std::string SearchUrlFor(const std::string& engine, const std::string& query) {
  const std::string escaped = EscapeQuery(query);
  if (engine == "google") {
    return "https://www.google.com/search?q=" + escaped;
  }
  if (engine == "bing") {
    return "https://www.bing.com/search?q=" + escaped;
  }
  return "https://duckduckgo.com/?q=" + escaped;
}

}  // namespace

std::string EscapeQuery(const std::string& input) {
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

std::string NormalizeNavigationInput(const std::string& input) {
  return NormalizeNavigationInput(input, "duckduckgo");
}

std::string NormalizeNavigationInput(const std::string& input, const std::string& searchEngine) {
  const std::string value = Trim(input);
  if (value.empty()) {
    return "fubuki://newtab/";
  }
  if (HasScheme(value)) {
    return value;
  }
  if (LooksLikeHost(value)) {
    return "https://" + value;
  }
  return SearchUrlFor(searchEngine, value);
}

}  // namespace fubuki
