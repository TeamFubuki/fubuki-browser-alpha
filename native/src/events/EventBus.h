#pragma once

#include <functional>
#include <map>
#include <mutex>
#include <string>
#include <vector>

#include "browser/Tab.h"

namespace fubuki {

enum class EventType {
  TabCreated,
  TabUpdated,
  TabClosed,
  TabActivated,
  NavigationStarted,
  NavigationFinished,
  NavigationFailed,
  AppStateChanged,
};

struct Event {
  EventType type;
  std::string name;
  Tab tab;
  std::string tabId;
  std::string message;
};

class EventBus {
 public:
  using Listener = std::function<void(const Event&)>;

  int Subscribe(EventType type, Listener listener);
  void Unsubscribe(EventType type, int token);
  void Publish(const Event& event);

 private:
  std::mutex mutex_;
  int nextToken_ = 1;
  std::map<EventType, std::map<int, Listener>> listeners_;
};

}  // namespace fubuki
