#include "browser/WindowIdPolicy.h"

#include <gtest/gtest.h>

#include <unordered_set>

namespace fubuki {
namespace {

TEST(WindowIdPolicyTest, PreservesPersistedWindowIdWithoutCallingFallback) {
  bool fallbackCalled = false;
  const std::string id = ResolveRestoredWindowId("window-persisted", [&] {
    fallbackCalled = true;
    return std::string("window-fallback");
  });

  EXPECT_EQ(id, "window-persisted");
  EXPECT_FALSE(fallbackCalled);
}

TEST(WindowIdPolicyTest, UsesFallbackWhenPersistedIdIsMissing) {
  EXPECT_EQ(ResolveRestoredWindowId("", [] { return "window-7"; }),
            "window-7");
}

TEST(WindowIdPolicyTest, GeneratedIdSkipsRestoredWindowIds) {
  int nextId = 1;
  const std::unordered_set<std::string> restoredIds = {"window-1",
                                                        "window-2"};

  EXPECT_EQ(NextAvailableWindowId(nextId, [&](const std::string &candidate) {
              return restoredIds.contains(candidate);
            }),
            "window-3");
  EXPECT_EQ(nextId, 4);
}

}  // namespace
}  // namespace fubuki
