#pragma once

#include <atomic>
#include <functional>
#include <string>
#include <thread>

namespace fubuki {

class AutomationIpcServer {
public:
  using Handler = std::function<std::string(const std::string &)>;

  explicit AutomationIpcServer(Handler handler);
  ~AutomationIpcServer();

  bool Start();
  void Stop();
  int Port() const {
    return port_;
  }

private:
  void Run();
  void HandleClient(int clientFd);

  Handler handler_;
  std::atomic<bool> running_{false};
  std::thread thread_;
  int serverFd_ = -1;
  int port_ = 42176;
};

}  // namespace fubuki
