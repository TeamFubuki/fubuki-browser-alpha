#include "bridge/FrostBridge.h"
#include "frost_ffi.h"

namespace {

std::string TakeFrostString(char *value) {
  if (!value) {
    return "";
  }
  std::string result(value);
  frost_engine_string_free(value);
  return result;
}

}  // namespace

namespace fubuki {

FrostBridge::FrostBridge() : handle_(frost_engine_new()) {
  StartWorker();
}

FrostBridge::FrostBridge(const std::string &profilePath)
    : handle_(frost_engine_new_with_store(
          profilePath.empty() ? nullptr : profilePath.c_str())) {
  StartWorker();
}

FrostBridge::~FrostBridge() {
  ShutdownAsync();
  if (handle_) {
    frost_engine_free(handle_);
    handle_ = nullptr;
  }
}

void FrostBridge::ShutdownAsync() {
  StopWorker();
}

void FrostBridge::StartWorker() {
  if (!handle_) {
    return;
  }
  worker_ = std::thread([this]() {
    for (;;) {
      AsyncRequest request;
      {
        std::unique_lock<std::mutex> lock(queueMutex_);
        queueCondition_.wait(lock,
                             [this]() { return stopping_ || !queue_.empty(); });
        if (stopping_) {
          return;
        }
        request = std::move(queue_.front());
        queue_.pop_front();
      }
      std::string response = ProcessJson(request.json);
      if (request.callback) {
        request.callback(std::move(response));
      }
    }
  });
}

void FrostBridge::StopWorker() {
  {
    std::lock_guard<std::mutex> lock(queueMutex_);
    stopping_ = true;
    queue_.clear();
  }
  queueCondition_.notify_all();
  if (worker_.joinable()) {
    worker_.join();
  }
}

std::string FrostBridge::ProcessJson(const std::string &requestJson) {
  if (!handle_) {
    return "{\"version\":0,\"ok\":false,\"kind\":\"error\",\"result\":\"FrostEngine is not available\"}";
  }
  return TakeFrostString(frost_engine_process_json(handle_, requestJson.c_str()));
}

bool FrostBridge::ProcessJsonAsync(
    std::string requestJson, std::function<void(std::string)> callback) {
  {
    std::lock_guard<std::mutex> lock(queueMutex_);
    if (!handle_ || stopping_ || queue_.size() >= kMaxPendingRequests) {
      return false;
    }
    queue_.push_back({std::move(requestJson), std::move(callback)});
  }
  queueCondition_.notify_one();
  return true;
}

bool FrostBridge::PollEventJson(std::string &eventJson) {
  if (!handle_) {
    return false;
  }
  eventJson = TakeFrostString(frost_engine_poll_event_json(handle_));
  return !eventJson.empty();
}

bool FrostBridge::PollHostCommandJson(std::string &commandJson) {
  if (!handle_) {
    return false;
  }
  commandJson = TakeFrostString(frost_engine_poll_host_command_json(handle_));
  return !commandJson.empty();
}

bool FrostBridge::PushHostEventJson(const std::string &eventJson) {
  return handle_ &&
         frost_engine_push_host_event_json(handle_, eventJson.c_str());
}

bool FrostBridge::PushHostCommandResultJson(const std::string &resultJson) {
  return handle_ &&
         frost_engine_push_host_command_result_json(handle_, resultJson.c_str());
}

bool FrostBridge::SetHostCommandNotifier(void (*callback)(void *),
                                         void *context) {
  return handle_ &&
         frost_engine_set_host_command_notify(handle_, callback, context);
}

bool FrostBridge::GrantExternal(const std::string &origin,
                                const std::string &capabilitiesJson) {
  return handle_ && frost_engine_grant_external(handle_, origin.c_str(),
                                                capabilitiesJson.c_str());
}

std::string FrostBridge::ProcessExternalJson(const std::string &commandJson) {
  if (!handle_) {
    return "{\"allowed\":false,\"error\":\"FrostEngine is not available\"}";
  }
  return TakeFrostString(
      frost_engine_process_external_json(handle_, commandJson.c_str()));
}

}  // namespace fubuki
