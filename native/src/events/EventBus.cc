#include "events/EventBus.h"

namespace fubuki {

int EventBus::Subscribe(EventType type, Listener listener) {
  std::lock_guard<std::mutex> lock(mutex_);
  const int token = nextToken_++;
  listeners_[type][token] = std::move(listener);
  return token;
}

void EventBus::Unsubscribe(EventType type, int token) {
  std::lock_guard<std::mutex> lock(mutex_);
  auto it = listeners_.find(type);
  if (it != listeners_.end()) {
    it->second.erase(token);
  }
}

void EventBus::Publish(const Event& event) {
  std::vector<Listener> listeners;
  {
    std::lock_guard<std::mutex> lock(mutex_);
    auto it = listeners_.find(event.type);
    if (it != listeners_.end()) {
      for (const auto& [_, listener] : it->second) {
        listeners.push_back(listener);
      }
    }
  }

  for (const auto& listener : listeners) {
    listener(event);
  }
}

}  // namespace fubuki
