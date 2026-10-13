#include <gtest/gtest.h>

#include "cef/PermissionPolicy.h"

using namespace fubuki;

TEST(PermissionPolicy, MapsAllSupportedBitsAndRejectsUnknownCombinations) {
  for (auto [mask, expected] :
       std::vector<std::pair<uint32_t, std::string>>{{1u << 1, "camera"},
                                                     {1u << 2, "camera"},
                                                     {1u << 8, "geolocation"},
                                                     {1u << 12, "microphone"},
                                                     {1u << 15, "notifications"},
                                                     {1u << 16, "keyboardLock"},
                                                     {1u << 17, "pointerLock"}}) {
    std::vector<std::string> names;
    ASSERT_TRUE(PermissionNames(mask, names));
    EXPECT_EQ(names, std::vector<std::string>{expected});
  }
  std::vector<std::string> names;
  EXPECT_TRUE(PermissionNames((1u << 1) | (1u << 2) | (1u << 12), names));
  EXPECT_EQ(names, (std::vector<std::string>{"camera", "microphone"}));
  for (unsigned bit = 0; bit < 32; ++bit) {
    const auto mask = 1u << bit;
    const auto known =
        (1u << 1) | (1u << 2) | (1u << 8) | (1u << 12) | (1u << 15) | (1u << 16) | (1u << 17);
    if (!(mask & known))
      EXPECT_FALSE(PermissionNames(mask | (1u << 2), names));
  }
  EXPECT_FALSE(PermissionNames(0, names));
  EXPECT_TRUE(names.empty());
}

TEST(PermissionPolicy, MediaAllowsOnlyDeviceCapture) {
  std::vector<std::string> names;
  EXPECT_TRUE(MediaPermissionNames(1, names));
  EXPECT_EQ(names, std::vector<std::string>{"microphone"});
  EXPECT_TRUE(MediaPermissionNames(2, names));
  EXPECT_EQ(names, std::vector<std::string>{"camera"});
  EXPECT_TRUE(MediaPermissionNames(3, names));
  EXPECT_EQ(names, (std::vector<std::string>{"microphone", "camera"}));
  EXPECT_FALSE(MediaPermissionNames(0, names));
  EXPECT_FALSE(MediaPermissionNames(5, names));
  EXPECT_FALSE(MediaPermissionNames(10, names));
}

TEST(PermissionCallbacks, CompletesOnceEvenWithReentryAndLateTimeout) {
  PermissionCallbacks callbacks;
  int calls = 0;
  const auto token = callbacks.Add("p", [&](const std::string& decision) {
    EXPECT_EQ(decision, "allow");
    ++calls;
    EXPECT_FALSE(callbacks.Resolve("p", "block"));
  });
  EXPECT_TRUE(callbacks.Resolve("p", "allow"));
  EXPECT_FALSE(callbacks.Resolve("p", "block", token));
  EXPECT_EQ(calls, 1);
}

TEST(PermissionCallbacks, OldTimeoutDoesNotResolveReusedId) {
  PermissionCallbacks callbacks;
  const auto old = callbacks.Add("p", [](auto&) {});
  EXPECT_EQ(callbacks.Add("p", [](auto&) {}), 0u);
  EXPECT_TRUE(callbacks.Remove("p"));
  int calls = 0;
  callbacks.Add("p", [&](auto&) { ++calls; });
  EXPECT_FALSE(callbacks.Resolve("p", "block", old));
  EXPECT_TRUE(callbacks.Resolve("p", "block"));
  EXPECT_EQ(calls, 1);
}

TEST(PermissionCallbacks, NavigationAndCloseCancelAllOnce) {
  PermissionCallbacks callbacks;
  int calls = 0;
  for (const auto* id : {"p", "media"})
    callbacks.Add(id, [&](const auto& decision) {
      EXPECT_EQ(decision, "ask");
      ++calls;
      callbacks.Cancel();
    });
  callbacks.Cancel();
  callbacks.Cancel();
  EXPECT_EQ(calls, 2);
  EXPECT_FALSE(callbacks.Resolve("p", "allow"));
}
