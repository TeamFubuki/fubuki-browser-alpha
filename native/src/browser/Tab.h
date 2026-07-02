#pragma once

#include <string>

#include "include/cef_browser.h"

namespace fubuki {

struct Tab {
  std::string id;
  std::string title;
  std::string url;
  std::string faviconUrl;
  std::string errorText;
  double zoomLevel = 0.0;
  bool isLoading = false;
  bool canGoBack = false;
  bool canGoForward = false;
  bool isActive = false;
  bool isPinned = false;
  CefRefPtr<CefBrowser> browser;
};

}  // namespace fubuki
