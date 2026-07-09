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

  // Returns the raw FrostEngine handle. Only used by FrostStore which
  // needs the raw handle for protocol delegation.
  void *RawHandle() const { return handle_; }

  std::string ProcessJson(const std::string &requestJson);
  bool PollEventJson(std::string &eventJson);
  bool PollHostCommandJson(std::string &commandJson);
  bool PushHostEventJson(const std::string &eventJson);
  bool PushHostCommandResultJson(const std::string &resultJson);

  // External / MCP automation boundary. Grants capabilities to a caller
  // origin and routes external commands through the engine policy layer.
  bool GrantExternal(const std::string &origin,
                     const std::string &capabilitiesJson);
  std::string ProcessExternalJson(const std::string &commandJson);

private:
  void *handle_ = nullptr;
};

}  // namespace fubuki
