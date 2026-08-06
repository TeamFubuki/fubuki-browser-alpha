#include <gtest/gtest.h>

#include <cctype>
#include <cmath>
#include <cstdlib>
#include <fstream>
#include <map>
#include <sstream>
#include <string>
#include <vector>

namespace {

struct JsonValue {
  enum class Type { kObject, kArray, kString, kNumber, kBool, kNull };
  Type type = Type::kNull;
  std::map<std::string, JsonValue> object;
  std::vector<JsonValue> array;
  std::string string;
  double number = 0;
  bool boolean = false;

  const JsonValue* Get(const std::string& key) const {
    if (type != Type::kObject) return nullptr;
    auto it = object.find(key);
    return it == object.end() ? nullptr : &it->second;
  }
};

class JsonParser {
 public:
  explicit JsonParser(std::string input) : input_(std::move(input)) {}

  bool Parse(JsonValue& value) {
    SkipWhitespace();
    if (!ParseValue(value)) return false;
    SkipWhitespace();
    return position_ == input_.size();
  }

 private:
  void SkipWhitespace() {
    while (position_ < input_.size() &&
           std::isspace(static_cast<unsigned char>(input_[position_]))) {
      ++position_;
    }
  }

  bool Consume(char expected) {
    SkipWhitespace();
    if (position_ >= input_.size() || input_[position_] != expected) return false;
    ++position_;
    return true;
  }

  bool ParseValue(JsonValue& value) {
    SkipWhitespace();
    if (position_ >= input_.size()) return false;
    switch (input_[position_]) {
      case '{': return ParseObject(value);
      case '[': return ParseArray(value);
      case '"':
        value.type = JsonValue::Type::kString;
        return ParseString(value.string);
      case 't': return ParseLiteral("true", value, true);
      case 'f': return ParseLiteral("false", value, false);
      case 'n': return ParseNull(value);
      default: return ParseNumber(value);
    }
  }

  bool ParseObject(JsonValue& value) {
    if (!Consume('{')) return false;
    value.type = JsonValue::Type::kObject;
    SkipWhitespace();
    if (position_ < input_.size() && input_[position_] == '}') {
      ++position_;
      return true;
    }
    while (true) {
      std::string key;
      if (!ParseString(key) || !Consume(':')) return false;
      JsonValue child;
      if (!ParseValue(child)) return false;
      value.object.emplace(std::move(key), std::move(child));
      SkipWhitespace();
      if (position_ < input_.size() && input_[position_] == '}') {
        ++position_;
        return true;
      }
      if (!Consume(',')) return false;
    }
  }

  bool ParseArray(JsonValue& value) {
    if (!Consume('[')) return false;
    value.type = JsonValue::Type::kArray;
    SkipWhitespace();
    if (position_ < input_.size() && input_[position_] == ']') {
      ++position_;
      return true;
    }
    while (true) {
      JsonValue child;
      if (!ParseValue(child)) return false;
      value.array.push_back(std::move(child));
      SkipWhitespace();
      if (position_ < input_.size() && input_[position_] == ']') {
        ++position_;
        return true;
      }
      if (!Consume(',')) return false;
    }
  }

  bool ParseString(std::string& value) {
    SkipWhitespace();
    if (position_ >= input_.size() || input_[position_] != '"') return false;
    ++position_;
    while (position_ < input_.size()) {
      const char current = input_[position_++];
      if (current == '"') return true;
      if (current == '\\') {
        if (position_ >= input_.size()) return false;
        const char escaped = input_[position_++];
        if (escaped == 'u') {
          if (position_ + 4 > input_.size()) return false;
          position_ += 4;
        } else if (escaped != '"' && escaped != '\\' && escaped != '/' &&
                   escaped != 'b' && escaped != 'f' && escaped != 'n' &&
                   escaped != 'r' && escaped != 't') {
          return false;
        }
        value.push_back(escaped);
      } else {
        if (static_cast<unsigned char>(current) < 0x20) return false;
        value.push_back(current);
      }
    }
    return false;
  }

  bool ParseLiteral(const char* literal, JsonValue& value, bool boolean) {
    const std::string token(literal);
    if (input_.compare(position_, token.size(), token) != 0) return false;
    position_ += token.size();
    value.type = JsonValue::Type::kBool;
    value.boolean = boolean;
    return true;
  }

  bool ParseNull(JsonValue& value) {
    if (input_.compare(position_, 4, "null") != 0) return false;
    position_ += 4;
    value.type = JsonValue::Type::kNull;
    return true;
  }

  bool ParseNumber(JsonValue& value) {
    SkipWhitespace();
    const char* begin = input_.c_str() + position_;
    char* end = nullptr;
    const double number = std::strtod(begin, &end);
    if (end == begin || !std::isfinite(number)) return false;
    position_ += static_cast<size_t>(end - begin);
    value.type = JsonValue::Type::kNumber;
    value.number = number;
    return true;
  }

  std::string input_;
  size_t position_ = 0;
};

std::string ReadFixture(const char* relativePath) {
  std::ifstream stream(std::string(FUBUKI_SOURCE_DIR) + "/" + relativePath);
  std::ostringstream content;
  content << stream.rdbuf();
  return content.str();
}

bool IsType(const JsonValue* value, JsonValue::Type type) {
  return value && value->type == type;
}

bool IsSupportedVersion(const JsonValue* value) {
  return IsType(value, JsonValue::Type::kNumber) && value->number == 0;
}

bool IsEnvelope(const JsonValue* value, const char* field) {
  return IsType(value, JsonValue::Type::kObject) &&
         IsSupportedVersion(value->Get("version")) &&
         IsType(value->Get(field), JsonValue::Type::kString);
}

bool IsScenario(const JsonValue& scenario) {
  if (scenario.type != JsonValue::Type::kObject ||
      !IsType(scenario.Get("name"), JsonValue::Type::kString)) {
    return false;
  }
  const JsonValue* request = scenario.Get("request");
  const JsonValue* response = scenario.Get("response");
  const JsonValue* event = scenario.Get("event");
  const JsonValue* command = scenario.Get("hostCommand");
  const JsonValue* result = scenario.Get("hostResult");
  return IsEnvelope(request, "method") && IsEnvelope(response, "kind") &&
         IsEnvelope(event, "event") && IsEnvelope(command, "command") &&
         IsEnvelope(result, "commandId") &&
         IsType(response->Get("ok"), JsonValue::Type::kBool) &&
         IsType(result->Get("ok"), JsonValue::Type::kBool);
}

TEST(ProtocolFixtureTest, RustAndCppReadTheSameTenScenarios) {
  JsonValue root;
  ASSERT_TRUE(JsonParser(ReadFixture("tests/fixtures/protocol/contract.json")).Parse(root));
  ASSERT_EQ(root.type, JsonValue::Type::kObject);
  ASSERT_TRUE(IsType(root.Get("version"), JsonValue::Type::kNumber));
  EXPECT_EQ(root.Get("version")->number, 0);
  const JsonValue* scenarios = root.Get("scenarios");
  ASSERT_TRUE(IsType(scenarios, JsonValue::Type::kArray));
  ASSERT_EQ(scenarios->array.size(), 10U);
  for (const JsonValue& scenario : scenarios->array) EXPECT_TRUE(IsScenario(scenario));
}

TEST(ProtocolFixtureTest, NegativeFixturesAreRejectedAtTheBoundary) {
  JsonValue root;
  ASSERT_TRUE(JsonParser(ReadFixture("tests/fixtures/protocol/negative.json")).Parse(root));
  const JsonValue* cases = root.Get("cases");
  ASSERT_TRUE(IsType(cases, JsonValue::Type::kArray));
  ASSERT_EQ(cases->array.size(), 4U);
  for (const JsonValue& testCase : cases->array) {
    ASSERT_TRUE(IsType(testCase.Get("payload"), JsonValue::Type::kString));
    JsonValue payload;
    const bool parsed = JsonParser(testCase.Get("payload")->string).Parse(payload);
    const std::string& name = testCase.Get("name")->string;
    if (name == "malformed-json") {
      EXPECT_FALSE(parsed);
    } else if (name == "unknown-version") {
      ASSERT_TRUE(parsed);
      EXPECT_FALSE(IsSupportedVersion(payload.Get("version")));
    } else if (name == "missing-field") {
      ASSERT_TRUE(parsed);
      const JsonValue* event = payload.Get("payload");
      EXPECT_FALSE(event && event->Get("tabId"));
    } else if (name == "wrong-type") {
      ASSERT_TRUE(parsed);
      EXPECT_FALSE(IsType(payload.Get("ok"), JsonValue::Type::kBool));
    } else {
      FAIL() << "unexpected negative fixture: " << name;
    }
  }
}

}  // namespace
