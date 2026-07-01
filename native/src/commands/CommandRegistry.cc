#include "commands/CommandRegistry.h"

namespace fubuki {

void CommandRegistry::Register(std::string id, Handler handler) {
  const std::string commandId = id;
  Register(std::move(id), commandId, "General", "", std::move(handler));
}

void CommandRegistry::Register(std::string id, std::string title, std::string category, std::string shortcut, Handler handler) {
  commands_[id] = {id, std::move(title), std::move(category), std::move(shortcut)};
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

CefRefPtr<CefListValue> CommandRegistry::List() const {
  auto list = CefListValue::Create();
  size_t index = 0;
  for (const auto& [_, command] : commands_) {
    auto item = CefDictionaryValue::Create();
    item->SetString("id", command.id);
    item->SetString("title", command.title);
    item->SetString("category", command.category);
    item->SetString("shortcut", command.shortcut);
    list->SetDictionary(index++, item);
  }
  return list;
}

}  // namespace fubuki
