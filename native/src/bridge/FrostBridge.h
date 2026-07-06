#pragma once

#include <memory>
#include <string>

namespace fubuki {

class FrostBridge {
public:
  FrostBridge();
  ~FrostBridge();

  FrostBridge(const FrostBridge &) = delete;
  FrostBridge &operator=(const FrostBridge &) = delete;

  bool IsAvailable() const {
    return handle_ != nullptr;
  }

  std::string ProcessJson(const std::string &requestJson);
  bool PollEventJson(std::string &eventJson);

private:
  void *handle_ = nullptr;
};

}  // namespace fubuki
