#include "automation/AutomationController.h"

#include <future>

#include "automation/AutomationIpcServer.h"
#include "automation/BrowserAutomation.h"
#include "automation/PageAutomation.h"
#include "browser/BrowserAppController.h"
#include "browser/BrowserWindow.h"
#include "include/base/cef_callback.h"
#include "include/cef_parser.h"
#include "include/wrapper/cef_closure_task.h"

namespace fubuki {

namespace {

std::string WriteJson(CefRefPtr<CefValue> value) {
  return CefWriteJSON(value, JSON_WRITER_DEFAULT).ToString();
}

CefRefPtr<CefDictionaryValue> Ok(CefRefPtr<CefValue> result) {
  auto response = CefDictionaryValue::Create();
  response->SetBool("ok", true);
  response->SetValue("result", result ? result : CefValue::Create());
  return response;
}

CefRefPtr<CefValue> DictValue(CefRefPtr<CefDictionaryValue> dict) {
  auto value = CefValue::Create();
  value->SetDictionary(dict);
  return value;
}

CefRefPtr<CefValue> StringValue(const std::string &text) {
  auto value = CefValue::Create();
  value->SetString(text);
  return value;
}

CefRefPtr<CefDictionaryValue> Params(CefRefPtr<CefDictionaryValue> request) {
  return request && request->HasKey("params") &&
                 request->GetType("params") == VTYPE_DICTIONARY
             ? request->GetDictionary("params")
             : CefDictionaryValue::Create();
}

}  // namespace

AutomationController::AutomationController(BrowserAppController &app)
    : app_(app) {}

AutomationController::~AutomationController() {
  if (ipcServer_) {
    ipcServer_->Stop();
  }
}

bool AutomationController::Enabled() const {
  return enabled_;
}

void AutomationController::RefreshFromSettings() {
  const bool next =
      app_.Store().Settings()->GetString("automation.mcp.enabled") == "on";
  if (next == enabled_) {
    return;
  }
  enabled_ = next;
  if (enabled_) {
    ipcServer_ = std::make_unique<AutomationIpcServer>(
        [this](const std::string &json) { return HandleRequest(json); });
    if (ipcServer_->Start()) {
      app_.Store().Log("info", "Automation IPC started on localhost:" +
                                   std::to_string(ipcServer_->Port()));
    } else {
      enabled_ = false;
      ipcServer_.reset();
      app_.Store().SetSetting("automation.mcp.enabled", "off");
      app_.Store().Log("error", "Automation IPC failed to start");
    }
  } else if (ipcServer_) {
    ipcServer_->Stop();
    ipcServer_.reset();
    app_.Store().Log("info", "Automation IPC stopped");
  }
}

std::string AutomationController::HandleRequest(const std::string &json) {
  std::promise<CefRefPtr<CefDictionaryValue>> promise;
  auto future = promise.get_future();
  CefPostTask(TID_UI, base::BindOnce(
                          [](AutomationController *controller,
                             std::string requestJson,
                             std::promise<CefRefPtr<CefDictionaryValue>> *out) {
                            CefRefPtr<CefDictionaryValue> response;
                            if (!controller->enabled_) {
                              response = controller->Error(
                                  "disabled", "MCP automation is disabled");
                            } else {
                              CefRefPtr<CefValue> parsed =
                                  CefParseJSON(requestJson, JSON_PARSER_RFC);
                              if (!parsed ||
                                  parsed->GetType() != VTYPE_DICTIONARY) {
                                response = controller->Error(
                                    "bad_request", "Expected a JSON object");
                              } else {
                                response =
                                    controller->Dispatch(parsed->GetDictionary());
                              }
                            }
                            out->set_value(response);
                          },
                          this, json, &promise));
  CefRefPtr<CefDictionaryValue> response = future.get();
  return WriteJson(DictValue(response));
}

CefRefPtr<CefDictionaryValue> AutomationController::Error(
    const std::string &code, const std::string &message) const {
  auto error = CefDictionaryValue::Create();
  error->SetString("code", code);
  error->SetString("message", message);
  auto response = CefDictionaryValue::Create();
  response->SetBool("ok", false);
  response->SetDictionary("error", error);
  return response;
}

CefRefPtr<CefDictionaryValue> AutomationController::Dispatch(
    CefRefPtr<CefDictionaryValue> request) {
  const std::string method = request->GetString("method");
  if (method.empty()) {
    return Error("bad_request", "Missing method");
  }
  if (auto *window = app_.ActiveWindow(); window && window->IsPrivate()) {
    Audit(method, "blocked:private_window");
    return Error("private_window", "Automation is disabled in Private Window");
  }

  BrowserAutomation browser(app_);
  PageAutomation page(app_);
  CefRefPtr<CefValue> result;
  const auto params = Params(request);

  if (method == "browser.snapshot") {
    result = browser.Snapshot();
  } else if (method == "tabs.list") {
    result = browser.ListTabs();
  } else if (method == "tabs.create") {
    result = browser.CreateTab(params);
  } else if (method == "tabs.navigate") {
    result = browser.Navigate(params);
  } else if (method == "tabs.activate") {
    result = browser.ActivateTab(params);
  } else if (method == "tabs.close") {
    result = browser.CloseTab(params);
  } else if (method == "tabs.reload") {
    result = browser.Reload(params);
  } else if (method == "tabs.goBack") {
    result = browser.GoBack(params);
  } else if (method == "tabs.goForward") {
    result = browser.GoForward(params);
  } else if (method == "page.getText") {
    result = page.GetText(params);
  } else if (method == "page.getHtml") {
    result = page.GetHtml(params);
  } else if (method == "page.screenshot") {
    result = page.Screenshot(params);
  } else if (method == "page.getAccessibilityTree") {
    result = page.GetAccessibilityTree(params);
  } else if (method == "page.click") {
    result = page.Click(params);
  } else if (method == "page.type") {
    result = page.Type(params);
  } else if (method == "page.press") {
    result = page.Press(params);
  } else if (method == "page.scroll") {
    result = page.Scroll(params);
  } else if (method == "page.find") {
    result = page.Find(params);
  } else if (method == "bookmarks.list") {
    result = browser.ListBookmarks();
  } else if (method == "history.list") {
    result = browser.ListHistory();
  } else if (method == "downloads.list") {
    result = browser.ListDownloads();
  } else if (method == "page.evaluate" || method == "cdp.send") {
    Audit(method, "blocked:forbidden_api");
    return Error("forbidden_api", "Raw JavaScript and raw CDP are not exposed");
  } else {
    return Error("unknown_method", "Unknown automation method: " + method);
  }

  Audit(method, "ok");
  return Ok(result ? result : StringValue(""));
}

void AutomationController::Audit(const std::string &method,
                                 const std::string &result) {
  app_.Store().Log("info", "MCP tool call: " + method + " " + result);
}

}  // namespace fubuki
