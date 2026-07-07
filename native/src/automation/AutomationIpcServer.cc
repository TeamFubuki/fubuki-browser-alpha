#include "automation/AutomationIpcServer.h"

#include <array>
#include <cerrno>
#include <cstring>

#include <arpa/inet.h>
#include <netinet/in.h>
#include <sys/socket.h>
#include <unistd.h>

namespace fubuki {

AutomationIpcServer::AutomationIpcServer(Handler handler)
    : handler_(std::move(handler)) {}

AutomationIpcServer::~AutomationIpcServer() {
  Stop();
}

bool AutomationIpcServer::Start() {
  if (running_) {
    return true;
  }
  serverFd_ = socket(AF_INET, SOCK_STREAM, 0);
  if (serverFd_ < 0) {
    return false;
  }
  int yes = 1;
  setsockopt(serverFd_, SOL_SOCKET, SO_REUSEADDR, &yes, sizeof(yes));

  sockaddr_in addr{};
  addr.sin_family = AF_INET;
  addr.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
  addr.sin_port = htons(static_cast<uint16_t>(port_));
  if (bind(serverFd_, reinterpret_cast<sockaddr *>(&addr), sizeof(addr)) < 0 ||
      listen(serverFd_, 8) < 0) {
    close(serverFd_);
    serverFd_ = -1;
    return false;
  }

  running_ = true;
  thread_ = std::thread([this] { Run(); });
  return true;
}

void AutomationIpcServer::Stop() {
  if (!running_.exchange(false)) {
    return;
  }
  if (serverFd_ >= 0) {
    shutdown(serverFd_, SHUT_RDWR);
    close(serverFd_);
    serverFd_ = -1;
  }
  if (thread_.joinable()) {
    thread_.join();
  }
}

void AutomationIpcServer::Run() {
  while (running_) {
    const int client = accept(serverFd_, nullptr, nullptr);
    if (client < 0) {
      if (running_) {
        continue;
      }
      break;
    }
    std::thread([this, client] { HandleClient(client); }).detach();
  }
}

void AutomationIpcServer::HandleClient(int clientFd) {
  std::string buffer;
  std::array<char, 4096> chunk{};
  while (running_) {
    const ssize_t n = recv(clientFd, chunk.data(), chunk.size(), 0);
    if (n <= 0) {
      break;
    }
    buffer.append(chunk.data(), static_cast<size_t>(n));
    size_t newline = std::string::npos;
    while ((newline = buffer.find('\n')) != std::string::npos) {
      const std::string request = buffer.substr(0, newline);
      buffer.erase(0, newline + 1);
      if (request.empty()) {
        continue;
      }
      std::string response = handler_(request);
      response.push_back('\n');
      send(clientFd, response.data(), response.size(), 0);
    }
  }
  close(clientFd);
}

}  // namespace fubuki
