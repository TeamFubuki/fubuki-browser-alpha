#pragma once

#include <cstddef>

namespace fubuki {

struct TabContextMenuState {
  bool can_close_other_tabs;
  bool can_close_to_right;
};

constexpr TabContextMenuState TabContextMenuStateFor(std::size_t tab_index,
                                                     std::size_t tab_count) {
  const bool valid_index = tab_index < tab_count;
  return {.can_close_other_tabs = valid_index && tab_count > 1,
          .can_close_to_right = valid_index && tab_index + 1 < tab_count};
}

}  // namespace fubuki
