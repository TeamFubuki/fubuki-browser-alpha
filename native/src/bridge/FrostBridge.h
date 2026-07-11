#pragma once

#include <condition_variable>
#include <deque>
#include <functional>
#include <memory>
#include <mutex>
#include <string>
#include <thread>

namespace fubuki {

class FrostBridge {
public:
  FrostBridge();
  explicit FrostBridge(const std::string &profilePath);
  ~FrostBridge();

  FrostBridge(const FrostBridge &) = delete;
  FrostBridge &operator=(const FrostBridge &) = delete;

  bool IsAvailable() const {
    return handle_ != nullptr;
  }

  // Returns the raw FrostEngine handle. Only used by FrostStore which
  // needs the raw handle for protocol delegation.
  void *RawHandle() const { return handle_; }

  std::string ProcessJson(const std::string &requestJson);
  // Enqueues a request on a single bounded worker instead of creating a
  // detached thread per bridge call. The callback runs on the worker thread.
  bool ProcessJsonAsync(std::string requestJson,
                        std::function<void(std::string)> callback);
  // Stops accepting asynchronous work and joins the worker. Safe to call
  // repeatedly; controllers use this before dependent members are destroyed.
  void ShutdownAsync();
  bool PollEventJson(std::string &eventJson);
  bool PollHostCommandJson(std::string &commandJson);
  bool PushHostEventJson(const std::string &eventJson);
  bool PushHostCommandResultJson(const std::string &resultJson);
  bool SetHostCommandNotifier(void (*callback)(void *), void *context);

  // External / MCP automation boundary. Grants capabilities to a caller
  // origin and routes external commands through the engine policy layer.
  bool GrantExternal(const std::string &origin,
                     const std::string &capabilitiesJson);
  std::string ProcessExternalJson(const std::string &commandJson);

private:
  struct AsyncRequest {
    std::string json;
    std::function<void(std::string)> callback;
  };

  void StartWorker();
  void StopWorker();

  void *handle_ = nullptr;
  std::mutex queueMutex_;
  std::condition_variable queueCondition_;
  std::deque<AsyncRequest> queue_;
  std::thread worker_;
  bool stopping_ = false;
  static constexpr size_t kMaxPendingRequests = 256;
};

}  // namespace fubuki
