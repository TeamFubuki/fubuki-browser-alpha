#pragma once

#include <algorithm>
#include <optional>
#include <string>

namespace fubuki {

class SidebarLayoutState {
 public:
  bool ApplyVisibility(const std::string& value) {
    if (value != "show" && value != "hide") {
      return false;
    }
    visible_ = value == "show";
    return true;
  }

  bool ApplyWidth(const std::string& value, double minimum, double maximum) {
    try {
      width_ = std::clamp(std::stod(value), minimum, maximum);
      return true;
    } catch (...) {
      return false;
    }
  }

  void ApplyWidth(double value, double minimum, double maximum) {
    width_ = std::clamp(value, minimum, maximum);
  }

  bool Visible(bool persisted) const { return visible_.value_or(persisted); }
  double Width(double persisted) const { return width_.value_or(persisted); }
  void UsePersistedVisibility() { visible_.reset(); }
  void UsePersistedWidth() { width_.reset(); }

 private:
  std::optional<bool> visible_;
  std::optional<double> width_;
};

}  // namespace fubuki
