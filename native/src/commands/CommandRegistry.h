#pragma once

#include <functional>
#include <map>
#include <string>

#include "include/cef_values.h"

namespace fubuki {

class CommandRegistry {
public:
  using Handler =
      std::function<CefRefPtr<CefValue>(CefRefPtr<CefDictionaryValue>)>;

  struct CommandInfo {
    std::string id;
    std::string title;
    std::string category;
    std::string shortcut;
  };

  void Register(std::string id, Handler handler);
  void Register(std::string id, std::string title, std::string category,
                std::string shortcut, Handler handler);
  bool Has(const std::string &id) const;
  CefRefPtr<CefValue> Execute(const std::string &id,
                              CefRefPtr<CefDictionaryValue> args) const;
  CefRefPtr<CefListValue> List() const;

private:
  std::map<std::string, Handler> handlers_;
  std::map<std::string, CommandInfo> commands_;
};

}  // namespace fubuki
