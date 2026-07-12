#include "browser/TabContextMenuState.h"

#include <gtest/gtest.h>

namespace fubuki {

TEST(TabContextMenuStateTest, DisablesGroupActionsForOnlyTab) {
  const auto state = TabContextMenuStateFor(0, 1);
  EXPECT_FALSE(state.can_close_other_tabs);
  EXPECT_FALSE(state.can_close_to_right);
}

TEST(TabContextMenuStateTest, EnablesOnlyActionsSupportedByPosition) {
  EXPECT_TRUE(TabContextMenuStateFor(0, 2).can_close_other_tabs);
  EXPECT_TRUE(TabContextMenuStateFor(0, 2).can_close_to_right);
  EXPECT_TRUE(TabContextMenuStateFor(1, 2).can_close_other_tabs);
  EXPECT_FALSE(TabContextMenuStateFor(1, 2).can_close_to_right);
}

TEST(TabContextMenuStateTest, RejectsUnknownTabIndex) {
  const auto state = TabContextMenuStateFor(2, 2);
  EXPECT_FALSE(state.can_close_other_tabs);
  EXPECT_FALSE(state.can_close_to_right);
}

}  // namespace fubuki
