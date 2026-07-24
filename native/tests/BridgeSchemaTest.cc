#include <gtest/gtest.h>

#include <string>

#include "bridge/BridgeSchema.h"

namespace {

using fubuki::bridge::Params;
using fubuki::bridge::Value;
using fubuki::bridge::ValueType;

Value String(std::string value) {
  return {ValueType::kString, value, value.size(), 0.0};
}

Value Bool() {
  return {ValueType::kBool};
}

Value Number(double value) {
  return {ValueType::kNumber, "", 0, value};
}

Value Dictionary() {
  return {ValueType::kDictionary};
}

bool IsValid(const std::string& method, const Params& params = {}) {
  return !fubuki::bridge::Validate(method, params).has_value();
}

TEST(BridgeSchemaTest, RejectsUnknownMethod) {
  EXPECT_FALSE(IsValid("tabs.destroyEverything"));
}

TEST(BridgeSchemaTest, AcceptsEmptySnapshot) {
  EXPECT_TRUE(IsValid("app.snapshot"));
}

TEST(BridgeSchemaTest, RejectsUnknownSnapshotParameter) {
  EXPECT_FALSE(IsValid("app.snapshot", {{"extra", String("value")}}));
}

TEST(BridgeSchemaTest, AcceptsValidCreateTab) {
  EXPECT_TRUE(IsValid("tabs.create", {{"url", String("https://example.com")}, {"active", Bool()}}));
}

TEST(BridgeSchemaTest, RejectsOversizedCreateTabUrl) {
  EXPECT_FALSE(IsValid("tabs.create", {{"url", String(std::string(8193, 'x'))}}));
}

TEST(BridgeSchemaTest, RejectsMissingTabId) {
  EXPECT_FALSE(IsValid("tabs.activate"));
}

TEST(BridgeSchemaTest, RejectsTabIdWithWrongType) {
  EXPECT_FALSE(IsValid("tabs.activate", {{"tabId", Number(1)}}));
}

TEST(BridgeSchemaTest, RejectsEmptyTabId) {
  EXPECT_FALSE(IsValid("tabs.close", {{"tabId", String("")}}));
}

TEST(BridgeSchemaTest, RequiresPinnedValue) {
  EXPECT_FALSE(IsValid("tabs.pin", {{"tabId", String("tab-1")}}));
}

TEST(BridgeSchemaTest, RejectsPinnedValueWithWrongType) {
  EXPECT_FALSE(IsValid("tabs.pin", {{"tabId", String("tab-1")}, {"pinned", String("true")}}));
}

TEST(BridgeSchemaTest, RejectsNegativeTabIndex) {
  EXPECT_FALSE(IsValid("tabs.move", {{"tabId", String("tab-1")}, {"toIndex", Number(-1)}}));
}

TEST(BridgeSchemaTest, RejectsOversizedTabIndex) {
  EXPECT_FALSE(IsValid("tabs.move", {{"tabId", String("tab-1")}, {"toIndex", Number(10001)}}));
}

TEST(BridgeSchemaTest, RequiresNavigationInput) {
  EXPECT_FALSE(IsValid("tabs.navigate", {{"tabId", String("tab-1")}}));
}

TEST(BridgeSchemaTest, RejectsOversizedNavigationInput) {
  EXPECT_FALSE(IsValid("tabs.navigate",
                       {{"tabId", String("tab-1")}, {"input", String(std::string(8193, 'x'))}}));
}

TEST(BridgeSchemaTest, RequiresFindQuery) {
  EXPECT_FALSE(IsValid("page.find"));
}

TEST(BridgeSchemaTest, RejectsFindDirectionWithWrongType) {
  EXPECT_FALSE(IsValid("page.find", {{"query", String("fubuki")}, {"forward", Number(1)}}));
}

TEST(BridgeSchemaTest, RequiresBookmarkTitle) {
  EXPECT_FALSE(IsValid("bookmarks.save", {{"url", String("https://example.com")}}));
}

TEST(BridgeSchemaTest, RejectsOversizedBookmarkUrl) {
  EXPECT_FALSE(IsValid("bookmarks.save",
                       {{"title", String("Example")}, {"url", String(std::string(8193, 'x'))}}));
}

TEST(BridgeSchemaTest, RejectsUnsupportedHistoryRange) {
  EXPECT_FALSE(IsValid("history.clearRange", {{"range", String("forever")}}));
}

TEST(BridgeSchemaTest, RejectsOversizedDownloadPath) {
  EXPECT_FALSE(IsValid("downloads.open", {{"path", String(std::string(4097, 'x'))}}));
}

TEST(BridgeSchemaTest, RejectsOversizedSettingValue) {
  EXPECT_FALSE(IsValid("settings.set",
                       {{"key", String("theme")}, {"value", String(std::string(16385, 'x'))}}));
}

TEST(BridgeSchemaTest, RejectsUnknownSettingParameter) {
  EXPECT_FALSE(IsValid("settings.get", {{"key", String("theme")}, {"secret", String("value")}}));
}

TEST(BridgeSchemaTest, RejectsUnsupportedPermissionValue) {
  EXPECT_FALSE(IsValid("permissions.set", {{"origin", String("https://example.com")},
                                           {"permission", String("camera")},
                                           {"value", String("always")}}));
}

TEST(BridgeSchemaTest, RejectsSidebarWidthBelowMinimum) {
  EXPECT_FALSE(IsValid("ui.setSidebarWidth", {{"width", Number(159)}}));
}

TEST(BridgeSchemaTest, RejectsSidebarWidthAboveMaximum) {
  EXPECT_FALSE(IsValid("ui.setSidebarWidth", {{"width", Number(801)}}));
}

TEST(BridgeSchemaTest, AcceptsSidebarWidthAtBoundary) {
  EXPECT_TRUE(IsValid("ui.setSidebarWidth", {{"width", Number(800)}}));
}

TEST(BridgeSchemaTest, RequiresOverlayActiveFlag) {
  EXPECT_FALSE(IsValid("ui.setOverlayActive", {{"width", Number(392)}}));
}

TEST(BridgeSchemaTest, RejectsOverlayWidthBelowMinimum) {
  EXPECT_FALSE(IsValid("ui.setOverlayActive", {{"active", Bool()}, {"width", Number(99)}}));
}

TEST(BridgeSchemaTest, RejectsOverlayHeightAboveMaximum) {
  EXPECT_FALSE(IsValid("ui.setOverlayActive", {{"active", Bool()}, {"height", Number(2001)}}));
}

TEST(BridgeSchemaTest, RequiresCommandIdentifier) {
  EXPECT_FALSE(IsValid("commands.execute", {{"args", Dictionary()}}));
}

TEST(BridgeSchemaTest, RejectsCommandArgumentsWithWrongType) {
  EXPECT_FALSE(IsValid("commands.execute",
                       {{"id", String("tabs.create")}, {"args", String("not-an-object")}}));
}

TEST(BridgeSchemaTest, AcceptsCommandWithDictionaryArguments) {
  EXPECT_TRUE(IsValid("commands.execute", {{"id", String("tabs.create")}, {"args", Dictionary()}}));
}

TEST(BridgeSchemaTest, AcceptsOptionalHomeIdentifiers) {
  EXPECT_TRUE(IsValid("tabs.home", {{"tabId", String("tab-1")}, {"windowId", String("window-1")}}));
}

TEST(BridgeSchemaTest, AcceptsOptionalWindowCloseIdentifier) {
  EXPECT_TRUE(IsValid("windows.close", {{"windowId", String("window-1")}}));
}

}  // namespace
