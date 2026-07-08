#include "bridge/FrostBridge.h"

namespace {

extern "C" {
void *frost_engine_new();
void *frost_engine_new_with_store(const char *path);
void frost_engine_free(void *handle);
char *frost_engine_process_json(void *handle, const char *request_json);
char *frost_engine_poll_event_json(void *handle);
char *frost_engine_poll_host_command_json(void *handle);
bool frost_engine_push_host_event_json(void *handle, const char *event_json);
bool frost_engine_push_host_command_result_json(void *handle,
                                                const char *result_json);
void frost_engine_string_free(char *value);
}

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

FrostBridge::FrostBridge() : handle_(frost_engine_new()) {}

FrostBridge::FrostBridge(const std::string &profilePath)
    : handle_(frost_engine_new_with_store(
          profilePath.empty() ? nullptr : profilePath.c_str())) {}

FrostBridge::~FrostBridge() {
  if (handle_) {
    frost_engine_free(handle_);
    handle_ = nullptr;
  }
}

std::string FrostBridge::ProcessJson(const std::string &requestJson) {
  if (!handle_) {
    return "{\"version\":0,\"ok\":false,\"kind\":\"error\",\"result\":\"FrostEngine is not available\"}";
  }
  return TakeFrostString(frost_engine_process_json(handle_, requestJson.c_str()));
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

}  // namespace fubuki
