#pragma once

#include <memory>
#include <string>

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

  std::string ProcessJson(const std::string &requestJson);
  bool PollEventJson(std::string &eventJson);
  bool PollHostCommandJson(std::string &commandJson);
  bool PushHostEventJson(const std::string &eventJson);
  bool PushHostCommandResultJson(const std::string &resultJson);

private:
  void *handle_ = nullptr;
};

}  // namespace fubuki
