#include <gtest/gtest.h>

#include "utils/JsonUtils.h"

namespace {

TEST(JsonEscapeTest, WrapsAndEscapesBasicString) {
  EXPECT_EQ(fubuki::JsonEscape("hello"), "\"hello\"");
}

TEST(JsonEscapeTest, EscapesQuoteAndBackslash) {
  EXPECT_EQ(fubuki::JsonEscape("a\"b\\c"), "\"a\\\"b\\\\c\"");
}

TEST(JsonEscapeTest, EscapesControlCharacters) {
  EXPECT_EQ(fubuki::JsonEscape("line1\nline2\ttab"), "\"line1\\nline2\\ttab\"");
  EXPECT_EQ(fubuki::JsonEscape("a\rb"), "\"a\\rb\"");
  EXPECT_EQ(fubuki::JsonEscape("\x01"), "\"\\u0001\"");
}

TEST(JsonEscapeTest, EmptyString) {
  EXPECT_EQ(fubuki::JsonEscape(""), "\"\"");
}

TEST(JsonEscapeTest, ProducesValidJsonWhenEmbeddedInObject) {
  // A title with a quote must not break the surrounding JSON object.
  std::string json = "{\"title\":" + fubuki::JsonEscape("she said \"hi\"") + "}";
  EXPECT_EQ(json, "{\"title\":\"she said \\\"hi\\\"\"}");
}

}  // namespace
