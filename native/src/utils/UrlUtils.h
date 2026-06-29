#pragma once

#include <string>

namespace fubuki {

std::string NormalizeNavigationInput(const std::string& input);
std::string NormalizeNavigationInput(const std::string& input, const std::string& searchEngine);
std::string NormalizeNavigationInput(const std::string& input,
                                     const std::string& searchEngine,
                                     const std::string& customSearchUrl);
std::string EscapeQuery(const std::string& input);

}  // namespace fubuki
