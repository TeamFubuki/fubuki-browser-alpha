#include "bridge/BridgeSchema.h"

#include <algorithm>
#include <cmath>
#include <string_view>
#include <unordered_set>

namespace fubuki::bridge {
namespace {

constexpr size_t kMaxMethodLength = 128;
constexpr size_t kMaxIdLength = 256;
constexpr size_t kMaxUrlLength = 8192;
constexpr size_t kMaxPathLength = 4096;
constexpr size_t kMaxTextLength = 4096;
constexpr size_t kMaxSettingValueLength = 16384;

Field String(std::string name, bool required = false, size_t maxLength = kMaxTextLength) {
  return {std::move(name), ValueType::kString, required, maxLength};
}

Field Bool(std::string name, bool required = false) {
  return {std::move(name), ValueType::kBool, required};
}

Field Number(std::string name, bool required, double minimum, double maximum) {
  Field field{std::move(name), ValueType::kNumber, required};
  field.minimum = minimum;
  field.maximum = maximum;
  return field;
}

Field Dictionary(std::string name, bool required = false) {
  return {std::move(name), ValueType::kDictionary, required};
}

Field OneOf(std::string name, std::vector<std::string> values, bool required = false) {
  Field field = String(std::move(name), required, 64);
  field.allowedValues = std::move(values);
  return field;
}

Method Empty() {
  return {};
}

const std::unordered_map<std::string, Method>& Methods() {
  static const auto methods = [] {
    std::unordered_map<std::string, Method> schemas;
    for (std::string_view name : {"app.snapshot",      "tabs.list",
                                  "tabs.activateNext", "tabs.activatePrevious",
                                  "tabs.reopenClosed", "windows.create",
                                  "windows.list",      "windows.createPrivate",
                                  "windows.close",     "windows.reopenClosed",
                                  "page.zoomIn",       "page.zoomOut",
                                  "page.zoomReset",    "page.print",
                                  "page.viewSource",   "bookmarks.addActive",
                                  "bookmarks.list",    "history.list",
                                  "history.clear",     "downloads.list",
                                  "downloads.clear",   "bookmarks.clear",
                                  "app.openDevTools",  "commands.list"}) {
      schemas.emplace(std::string(name), Empty());
    }

    schemas["tabs.home"] = {
        {String("tabId", false, kMaxIdLength), String("windowId", false, kMaxIdLength)}};
    schemas["windows.close"] = {{String("windowId", false, kMaxIdLength)}};

    schemas["tabs.create"] = {{String("url", false, kMaxUrlLength), Bool("active"),
                               String("windowId", false, kMaxIdLength)}};
    for (std::string_view name :
         {"tabs.activate", "tabs.close", "tabs.duplicate", "tabs.closeOther", "tabs.closeToRight",
          "tabs.reload", "tabs.stop", "tabs.goBack", "tabs.goForward"}) {
      schemas.emplace(std::string(name), Method{{String("tabId", true, kMaxIdLength)}});
    }
    schemas["tabs.pin"] = {{String("tabId", true, kMaxIdLength), Bool("pinned", true)}};
    schemas["tabs.move"] = {
        {String("tabId", true, kMaxIdLength), Number("toIndex", true, 0, 10000)}};
    schemas["tabs.moveToNewWindow"] = {
        {String("tabId", true, kMaxIdLength), String("windowId", false, kMaxIdLength)}};
    schemas["tabs.navigate"] = {
        {String("tabId", true, kMaxIdLength), String("input", true, kMaxUrlLength)}};

    schemas["page.find"] = {{String("query", true, kMaxTextLength), Bool("forward")}};
    schemas["page.stopFinding"] = {{Bool("clear")}};

    schemas["bookmarks.save"] = {{String("title", true, kMaxTextLength),
                                  String("url", true, kMaxUrlLength),
                                  String("faviconUrl", false, kMaxUrlLength)}};
    schemas["bookmarks.remove"] = {{String("url", true, kMaxUrlLength)}};
    schemas["history.remove"] = {{String("url", true, kMaxUrlLength)}};
    schemas["history.clearRange"] = {{OneOf("range", {"lastHour", "today", "all"}, true)}};
    schemas["downloads.remove"] = {
        {String("url", false, kMaxUrlLength), String("path", false, kMaxPathLength)}};
    schemas["downloads.open"] = {{String("path", true, kMaxPathLength)}};
    schemas["downloads.reveal"] = {{String("path", true, kMaxPathLength)}};
    schemas["data.clear"] = {{OneOf(
        "target", {"history", "cookies", "cache", "downloads", "siteData", "bookmarks", "all"})}};
    schemas["settings.get"] = {{String("key", true, kMaxIdLength)}};
    schemas["settings.set"] = {
        {String("key", true, kMaxIdLength), String("value", true, kMaxSettingValueLength)}};
    schemas["settings.reset"] = {{String("key", true, kMaxIdLength)}};
    schemas["permissions.set"] = {{String("origin", true, kMaxUrlLength),
                                   String("permission", true, kMaxIdLength),
                                   OneOf("value", {"ask", "allow", "deny"}, true)}};
    schemas["ui.setSidebarWidth"] = {{Number("width", true, 160, 800)}};
    schemas["ui.setOverlayActive"] = {{Bool("active", true), Number("width", false, 100, 2000),
                                       Number("height", false, 100, 2000)}};
    schemas["commands.execute"] = {{String("id", true, kMaxMethodLength), Dictionary("args")}};
    return schemas;
  }();
  return methods;
}

std::string Prefix(const std::string& method, const std::string& field) {
  return "Invalid bridge request for method '" + method + "', field '" + field + "': ";
}

}  // namespace

bool IsKnownMethod(const std::string& method) {
  return method.size() <= kMaxMethodLength && Methods().contains(method);
}

std::optional<std::string> Validate(const std::string& method, const Params& params) {
  const auto methodIt = Methods().find(method);
  if (methodIt == Methods().end()) {
    return "Unknown bridge method '" + method + "'";
  }

  const Method& schema = methodIt->second;
  std::unordered_set<std::string> knownFields;
  for (const Field& field : schema.fields) {
    knownFields.insert(field.name);
    const auto valueIt = params.find(field.name);
    if (valueIt == params.end()) {
      if (field.required) {
        return Prefix(method, field.name) + "is required";
      }
      continue;
    }

    const Value& value = valueIt->second;
    if (value.type != field.type) {
      return Prefix(method, field.name) + "has an invalid type";
    }
    if (field.type == ValueType::kString) {
      if (value.stringLength == 0 && field.required) {
        return Prefix(method, field.name) + "must not be empty";
      }
      if (field.maxLength > 0 && value.stringLength > field.maxLength) {
        return Prefix(method, field.name) + "is too long";
      }
      if (!field.allowedValues.empty() &&
          std::find(field.allowedValues.begin(), field.allowedValues.end(), value.stringValue) ==
              field.allowedValues.end()) {
        return Prefix(method, field.name) + "has an unsupported value";
      }
    }
    if (field.type == ValueType::kNumber) {
      if (!std::isfinite(value.number) || (field.minimum && value.number < *field.minimum) ||
          (field.maximum && value.number > *field.maximum)) {
        return Prefix(method, field.name) + "is outside the supported range";
      }
    }
  }

  if (!schema.allowUnknownFields) {
    for (const auto& [field, _] : params) {
      if (!knownFields.contains(field)) {
        return Prefix(method, field) + "is not supported";
      }
    }
  }
  return std::nullopt;
}

}  // namespace fubuki::bridge
