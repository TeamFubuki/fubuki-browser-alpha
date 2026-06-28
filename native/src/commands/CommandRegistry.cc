#include "commands/CommandRegistry.h"

namespace fubuki {

void CommandRegistry::Register(std::string id, Handler handler) {
  handlers_[std::move(id)] = std::move(handler);
}

bool CommandRegistry::Has(const std::string& id) const {
  return handlers_.contains(id);
}

CefRefPtr<CefValue> CommandRegistry::Execute(const std::string& id, CefRefPtr<CefDictionaryValue> args) const {
  auto it = handlers_.find(id);
  if (it == handlers_.end()) {
    auto value = CefValue::Create();
    value->SetNull();
    return value;
  }
  return it->second(args);
}

}  // namespace fubuki
