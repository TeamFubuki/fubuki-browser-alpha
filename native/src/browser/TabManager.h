#pragma once

#include <optional>
#include <string>
#include <vector>

#include "browser/Tab.h"
#include "events/EventBus.h"

namespace fubuki {

class TabManager {
public:
  TabManager(EventBus &eventBus, std::string windowId);

  Tab &CreateTab(const std::string &url, bool active);
  // Creates a tab with an explicit id. Used when the id is owned by an
  // external authority (e.g. FrostEngine HostCommand page.create) so that
  // host-side and engine-side tab ids stay in sync.
  Tab &CreateTab(const std::string &url, bool active, const std::string &tabId);
  // Removes a tab without selecting a successor. FrostEngine owns successor
  // selection and includes it in the page.close host command.
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
  std::string NextId();
  void EnsureActiveTab();

  EventBus &eventBus_;
  std::string windowId_;
  int nextId_ = 1;
  std::vector<Tab> tabs_;
  std::string activeTabId_;
};

}  // namespace fubuki
