#include "browser/SidebarLayoutState.h"

#include <gtest/gtest.h>

namespace fubuki {

TEST(SidebarLayoutStateTest, AppliedVisibilityOverridesStalePersistence) {
  SidebarLayoutState state;
  ASSERT_TRUE(state.ApplyVisibility("hide"));
  EXPECT_FALSE(state.Visible(true));
  ASSERT_TRUE(state.ApplyVisibility("show"));
  EXPECT_TRUE(state.Visible(false));
}

TEST(SidebarLayoutStateTest, RejectsInvalidVisibilityWithoutChangingState) {
  SidebarLayoutState state;
  ASSERT_TRUE(state.ApplyVisibility("hide"));
  EXPECT_FALSE(state.ApplyVisibility("invalid"));
  EXPECT_FALSE(state.Visible(true));
}

TEST(SidebarLayoutStateTest, LiveWidthSurvivesVisibilityChanges) {
  SidebarLayoutState state;
  ASSERT_TRUE(state.ApplyWidth("240", 168.0, 280.0));
  ASSERT_TRUE(state.ApplyVisibility("hide"));
  ASSERT_TRUE(state.ApplyVisibility("show"));
  EXPECT_DOUBLE_EQ(state.Width(196.0), 240.0);
}

TEST(SidebarLayoutStateTest, PersistedValuesResumeAfterReset) {
  SidebarLayoutState state;
  ASSERT_TRUE(state.ApplyVisibility("hide"));
  ASSERT_TRUE(state.ApplyWidth("999", 168.0, 280.0));
  state.UsePersistedVisibility();
  state.UsePersistedWidth();
  EXPECT_TRUE(state.Visible(true));
  EXPECT_DOUBLE_EQ(state.Width(196.0), 196.0);
}

}  // namespace fubuki
