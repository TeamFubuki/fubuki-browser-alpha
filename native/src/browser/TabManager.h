#pragma once

#include <optional>
#include <string>
#include <vector>

#include "browser/Tab.h"
#include "events/EventBus.h"

namespace fubuki {

class TabManager {
 public:
  explicit TabManager(EventBus& eventBus);

  Tab& CreateTab(const std::string& url, bool active);
  bool CloseTab(const std::string& tabId);
  bool ActivateTab(const std::string& tabId);
  bool ActivateNext();
  bool ActivatePrevious();
  Tab* GetTab(const std::string& tabId);
  Tab* GetActiveTab();
  std::vector<Tab> GetTabs() const;
  std::string GetActiveTabId() const;
  void UpdateTab(const std::string& tabId, const Tab& patch);
  void SetBrowser(const std::string& tabId, CefRefPtr<CefBrowser> browser);

 private:
  std::string NextId();
  void EnsureActiveTab();

  EventBus& eventBus_;
  int nextId_ = 1;
  std::vector<Tab> tabs_;
  std::string activeTabId_;
};

}  // namespace fubuki
