#pragma once

#include <optional>
#include <string>
#include <vector>

#include "browser/Tab.h"
#include "events/EventBus.h"

namespace fubuki {

class TabManager {
public:
  explicit TabManager(EventBus &eventBus);

  // Creates a host page registry entry with an explicit Rust-owned tab ID.
  // C++ must never allocate logical tab IDs.
  Tab &CreateTab(const std::string &url, bool active, const std::string &tabId);
  bool CloseTab(const std::string &tabId);
  bool ActivateTab(const std::string &tabId);
  bool ActivateNext();
  bool ActivatePrevious();
  bool MoveTab(const std::string &tabId, size_t toIndex);
  bool SetPinned(const std::string &tabId, bool pinned);
  Tab *GetTab(const std::string &tabId);
  Tab *GetActiveTab();
  std::vector<Tab> GetTabs() const;
  std::string GetActiveTabId() const;
  void UpdateTab(const std::string &tabId, const Tab &patch);
  void SetBrowser(const std::string &tabId, CefRefPtr<CefBrowser> browser);

private:
  void EnsureActiveTab();

  EventBus &eventBus_;
  std::vector<Tab> tabs_;
  std::string activeTabId_;
};

}  // namespace fubuki
