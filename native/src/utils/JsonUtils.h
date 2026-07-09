#ifndef FUBUKI_JSON_UTILS_H_
#define FUBUKI_JSON_UTILS_H_

#include <cstdio>
#include <string>

namespace fubuki {

// Escapes a string into a JSON string literal, including the surrounding
// quotes. The mandatory escape set (\", \\ and the JSON whitespace controls)
// plus any other control character are encoded so the result is always valid
// JSON, even when titles/urls/paths contain quotes or backslashes.
inline std::string JsonEscape(const std::string &input) {
  std::string out;
  out.reserve(input.size() + 2);
  out.push_back('"');
  for (unsigned char c : input) {
    switch (c) {
      case '"':
        out += "\\\"";
        break;
      case '\\':
        out += "\\\\";
        break;
      case '\b':
        out += "\\b";
        break;
      case '\f':
        out += "\\f";
        break;
      case '\n':
        out += "\\n";
        break;
      case '\r':
        out += "\\r";
        break;
      case '\t':
        out += "\\t";
        break;
      default:
        if (c < 0x20) {
          char buf[8];
          std::snprintf(buf, sizeof(buf), "\\u%04x", c);
          out += buf;
        } else {
          out.push_back(static_cast<char>(c));
        }
    }
  }
  out.push_back('"');
  return out;
}

}  // namespace fubuki

#endif  // FUBUKI_JSON_UTILS_H_
