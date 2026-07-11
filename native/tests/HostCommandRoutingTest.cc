#include "browser/HostCommandRouting.h"

#include <gtest/gtest.h>

namespace fubuki {

TEST(HostCommandRoutingTest, ExplicitWindowWinsWhenTabIdsTemporarilyOverlap) {
  EXPECT_EQ(ResolveHostCommandWindowId("source-window", "destination-window"),
            "source-window");
}

TEST(HostCommandRoutingTest, FallsBackToCurrentTabOwnerForLegacyCommands) {
  EXPECT_EQ(ResolveHostCommandWindowId("", "owner-window"), "owner-window");
}

}  // namespace fubuki
