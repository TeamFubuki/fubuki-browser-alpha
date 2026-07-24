#include "bridge/FrostBridge.h"
#include "frost_ffi.h"

#include <filesystem>

namespace {

std::string TakeFrostString(char *value) {
  if (!value) {
    return "";
  }
  std::string result(value);
  frost_engine_string_free(value);
  return result;
}

void *OpenPersistentEngine(const std::string &databasePath) {
  if (databasePath.empty()) {
    return frost_engine_new_with_store(nullptr);
  }
  const std::filesystem::path path(databasePath);
  if (path.has_parent_path()) {
    std::error_code error;
    std::filesystem::create_directories(path.parent_path(), error);
    // Let SQLite report a structured error if directory creation failed.
  }
  return frost_engine_new_with_store(databasePath.c_str());
}

std::string UnavailableResponse(const std::string &initializationError) {
  const std::string error = initializationError.empty()
                                ? "{\"code\":\"engine_stopped\",\"message\":\"FrostEngine is not available\"}"
                                : initializationError;
  return "{\"version\":0,\"ok\":false,\"kind\":\"error\",\"result\":" +
         error + "}";
}

}  // namespace

namespace fubuki {

FrostBridge::FrostBridge() : handle_(frost_engine_new_in_memory()) {
  if (!handle_) {
    initializationErrorJson_ =
        TakeFrostString(frost_engine_take_last_error_json());
  }
}

FrostBridge::FrostBridge(const std::string &profilePath)
    : handle_(OpenPersistentEngine(profilePath)) {
  if (!handle_) {
    initializationErrorJson_ =
        TakeFrostString(frost_engine_take_last_error_json());
  }
}

FrostBridge::~FrostBridge() {
  if (handle_) {
    frost_engine_free(handle_);
    handle_ = nullptr;
  }
}

std::string FrostBridge::ProcessJson(const std::string &requestJson) {
  if (!handle_) {
    return UnavailableResponse(initializationErrorJson_);
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
