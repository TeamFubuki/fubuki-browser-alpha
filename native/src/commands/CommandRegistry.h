#pragma once

#include <functional>
#include <map>
#include <string>

#include "include/cef_values.h"

namespace fubuki {

class CommandRegistry {
 public:
  using Handler = std::function<CefRefPtr<CefValue>(CefRefPtr<CefDictionaryValue>)>;

  void Register(std::string id, Handler handler);
  bool Has(const std::string& id) const;
  CefRefPtr<CefValue> Execute(const std::string& id, CefRefPtr<CefDictionaryValue> args) const;

 private:
  std::map<std::string, Handler> handlers_;
};

}  // namespace fubuki
