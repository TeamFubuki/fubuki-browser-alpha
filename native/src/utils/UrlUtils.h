#pragma once

#include <string>

namespace fubuki {

std::string NormalizeNavigationInput(const std::string& input);
std::string EscapeQuery(const std::string& input);

}  // namespace fubuki
