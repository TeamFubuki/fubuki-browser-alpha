#pragma once

#include <cstddef>
#include <optional>
#include <string>
#include <unordered_map>
#include <vector>

namespace fubuki::bridge {

enum class ValueType { kString, kBool, kNumber, kDictionary, kList, kNull };

struct Value {
  ValueType type;
  std::string stringValue;
  size_t stringLength = 0;
  double number = 0.0;
};

using Params = std::unordered_map<std::string, Value>;

struct Field {
  std::string name;
  ValueType type;
  bool required = false;
  size_t maxLength = 0;
  std::optional<double> minimum;
  std::optional<double> maximum;
  std::vector<std::string> allowedValues;
};

struct Method {
  std::vector<Field> fields;
  bool allowUnknownFields = false;
};

// A schema error intentionally names only the method and field. Values are
// never included because bridge requests can contain paths, URLs, and settings.
std::optional<std::string> Validate(const std::string& method, const Params& params);
bool IsKnownMethod(const std::string& method);

}  // namespace fubuki::bridge
