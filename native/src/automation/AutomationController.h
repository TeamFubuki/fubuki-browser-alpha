#pragma once

#include <memory>
#include <string>

#include "include/cef_values.h"

namespace fubuki {

class AutomationIpcServer;
class BrowserAppController;

class AutomationController {
public:
  explicit AutomationController(BrowserAppController &app);
  ~AutomationController();

  void RefreshFromSettings();
  bool Enabled() const;
  std::string HandleRequest(const std::string &json);

private:
  CefRefPtr<CefDictionaryValue> Dispatch(CefRefPtr<CefDictionaryValue> request);
  CefRefPtr<CefDictionaryValue> Error(const std::string &code,
                                      const std::string &message) const;
  void Audit(const std::string &method, const std::string &result);

  BrowserAppController &app_;
  std::unique_ptr<AutomationIpcServer> ipcServer_;
  bool enabled_ = false;
};

}  // namespace fubuki
