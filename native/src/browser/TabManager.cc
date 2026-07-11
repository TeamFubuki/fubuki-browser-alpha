#include "browser/TabManager.h"

#include <algorithm>
#include <utility>

namespace fubuki {

TabManager::TabManager(EventBus &eventBus, std::string windowId)
    : eventBus_(eventBus), windowId_(std::move(windowId)) {}

Tab &TabManager::CreateTab(const std::string &url, bool active) {
  if (tabs_.empty()) {
    active = true;
  }
  if (active) {
    for (auto &tab : tabs_) {
      tab.isActive = false;
    }
  }

  Tab tab;
  tab.id = NextId();
  tab.title = "New Tab";
  tab.url = url;
  tab.isActive = active;
  tabs_.push_back(tab);

  if (active) {
    activeTabId_ = tabs_.back().id;
  }

  eventBus_.Publish({EventType::TabCreated, "tabs.created", tabs_.back(), windowId_,
                     tabs_.back().id, ""});
  if (active) {
    eventBus_.Publish({EventType::TabActivated, "tabs.activated", tabs_.back(),
                       windowId_, tabs_.back().id, ""});
  }
  return tabs_.back();
}

Tab &TabManager::CreateTab(const std::string &url, bool active,
                           const std::string &tabId) {
  if (active) {
    for (auto &tab : tabs_) {
      tab.isActive = false;
    }
  }

  Tab tab;
  tab.id = tabId.empty() ? NextId() : tabId;
  tab.title = "New Tab";
  tab.url = url;
  tab.isActive = active;
  tabs_.push_back(tab);

  if (active) {
    activeTabId_ = tabs_.back().id;
  }

  eventBus_.Publish({EventType::TabCreated, "tabs.created", tabs_.back(), windowId_,
                     tabs_.back().id, ""});
  if (active) {
    eventBus_.Publish({EventType::TabActivated, "tabs.activated", tabs_.back(),
                       windowId_, tabs_.back().id, ""});
  }
  return tabs_.back();
}

bool TabManager::CloseTab(const std::string &tabId) {
  const auto it = std::find_if(tabs_.begin(), tabs_.end(),
                               [&](const Tab &tab) { return tab.id == tabId; });
  if (it == tabs_.end()) {
    return false;
  }

  Tab closed = *it;
  tabs_.erase(it);
  eventBus_.Publish(
      {EventType::TabClosed, "tabs.closed", closed, windowId_, tabId, ""});

  if (closed.isActive) {
    activeTabId_.clear();
  }
  return true;
}

bool TabManager::ActivateTab(const std::string &tabId) {
  Tab *target = GetTab(tabId);
  if (!target) {
    return false;
  }

  for (auto &tab : tabs_) {
    tab.isActive = tab.id == tabId;
  }
  activeTabId_ = tabId;
  eventBus_.Publish(
      {EventType::TabActivated, "tabs.activated", *target, windowId_, tabId, ""});
  return true;
}

bool TabManager::ActivateNext() {
  if (tabs_.empty()) {
    return false;
  }
  auto it = std::find_if(tabs_.begin(), tabs_.end(),
                         [](const Tab &tab) { return tab.isActive; });
  const size_t index =
      it == tabs_.end()
          ? 0
          : static_cast<size_t>(std::distance(tabs_.begin(), it) + 1) %
                tabs_.size();
  return ActivateTab(tabs_[index].id);
}

bool TabManager::ActivatePrevious() {
  if (tabs_.empty()) {
    return false;
  }
  auto it = std::find_if(tabs_.begin(), tabs_.end(),
                         [](const Tab &tab) { return tab.isActive; });
  size_t index = 0;
  if (it != tabs_.end()) {
    const size_t current =
        static_cast<size_t>(std::distance(tabs_.begin(), it));
    index = current == 0 ? tabs_.size() - 1 : current - 1;
  }
  return ActivateTab(tabs_[index].id);
}

bool TabManager::MoveTab(const std::string &tabId, size_t toIndex) {
  auto it = std::find_if(tabs_.begin(), tabs_.end(),
                         [&](const Tab &tab) { return tab.id == tabId; });
  if (it == tabs_.end()) {
    return false;
  }
  toIndex = std::min(toIndex, tabs_.size() - 1);
  Tab moved = *it;
  tabs_.erase(it);
  tabs_.insert(tabs_.begin() + static_cast<std::ptrdiff_t>(toIndex), moved);
  eventBus_.Publish(
      {EventType::TabUpdated, "tabs.updated", moved, windowId_, tabId, "reordered"});
  return true;
}

bool TabManager::SetPinned(const std::string &tabId, bool pinned) {
  Tab *tab = GetTab(tabId);
  if (!tab) {
    return false;
  }
  tab->isPinned = pinned;
  eventBus_.Publish({EventType::TabUpdated, "tabs.updated", *tab, windowId_, tabId,
                     pinned ? "pinned" : "unpinned"});
  return true;
}

Tab *TabManager::GetTab(const std::string &tabId) {
  auto it = std::find_if(tabs_.begin(), tabs_.end(),
                         [&](const Tab &tab) { return tab.id == tabId; });
  return it == tabs_.end() ? nullptr : &(*it);
}

Tab *TabManager::GetActiveTab() {
  return activeTabId_.empty() ? nullptr : GetTab(activeTabId_);
}

std::vector<Tab> TabManager::GetTabs() const {
  return tabs_;
}

std::string TabManager::GetActiveTabId() const {
  return activeTabId_;
}

void TabManager::UpdateTab(const std::string &tabId, const Tab &patch) {
  Tab *tab = GetTab(tabId);
  if (!tab) {
    return;
  }
  tab->title = patch.title.empty() ? tab->title : patch.title;
  tab->url = patch.url.empty() ? tab->url : patch.url;
  tab->faviconUrl =
      patch.faviconUrl.empty() ? tab->faviconUrl : patch.faviconUrl;
  tab->errorText = patch.errorText;
  tab->zoomLevel = patch.zoomLevel;
  tab->isLoading = patch.isLoading;
  tab->canGoBack = patch.canGoBack;
  tab->canGoForward = patch.canGoForward;
  tab->isPinned = patch.isPinned;
  tab->isPendingPopup = patch.isPendingPopup;
  eventBus_.Publish(
      {EventType::TabUpdated, "tabs.updated", *tab, windowId_, tabId, ""});
}

void TabManager::SetBrowser(const std::string &tabId,
                            CefRefPtr<CefBrowser> browser) {
  if (auto *tab = GetTab(tabId)) {
    tab->browser = browser;
  }
}

std::string TabManager::NextId() {
  return "tab-" + std::to_string(nextId_++);
}

void TabManager::EnsureActiveTab() {
  if (!tabs_.empty() && activeTabId_.empty()) {
    ActivateTab(tabs_.front().id);
  }
}

}  // namespace fubuki
