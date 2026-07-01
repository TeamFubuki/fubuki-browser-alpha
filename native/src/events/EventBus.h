#pragma once

#include <functional>
#include <map>
#include <mutex>
#include <string>
#include <vector>

#include "browser/Tab.h"

namespace fubuki {

enum class EventType {
  WindowCreated,
  WindowClosed,
  WindowFocused,
  TabCreated,
  TabUpdated,
  TabClosed,
  TabActivated,
  NavigationStarted,
  NavigationFinished,
  NavigationFailed,
  BookmarkChanged,
  HistoryChanged,
  DownloadChanged,
  SettingChanged,
  PermissionChanged,
  AppStateChanged,
};

struct Event {
  EventType type;
  std::string name;
  Tab tab;
  std::string windowId;
  std::string tabId;
  std::string message;
};

class EventBus {
 public:
  using Listener = std::function<void(const Event&)>;

  int Subscribe(EventType type, Listener listener);
  void Unsubscribe(EventType type, int token);
  void Publish(const Event& event);
  std::vector<Event> RecentEvents() const;

 private:
  mutable std::mutex mutex_;
  int nextToken_ = 1;
  std::map<EventType, std::map<int, Listener>> listeners_;
  std::vector<Event> recentEvents_;
};

}  // namespace fubuki
