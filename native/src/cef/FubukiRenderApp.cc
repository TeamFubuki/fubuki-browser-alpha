#include "cef/FubukiRenderApp.h"

namespace fubuki {

namespace {

bool IsFubukiAppFrame(CefRefPtr<CefFrame> frame) {
  return frame && frame->GetURL().ToString().rfind("fubuki://app/", 0) == 0;
}

}  // namespace

void FubukiRenderApp::OnRegisterCustomSchemes(CefRawPtr<CefSchemeRegistrar> registrar) {
  registrar->AddCustomScheme("fubuki", CEF_SCHEME_OPTION_STANDARD | CEF_SCHEME_OPTION_SECURE |
                                           CEF_SCHEME_OPTION_CORS_ENABLED | CEF_SCHEME_OPTION_FETCH_ENABLED);
}

void FubukiRenderApp::OnWebKitInitialized() {
  CefMessageRouterConfig config;
  config.js_query_function = "cefQuery";
  config.js_cancel_function = "cefQueryCancel";
  rendererRouter_ = CefMessageRouterRendererSide::Create(config);
}

void FubukiRenderApp::OnContextCreated(CefRefPtr<CefBrowser> browser,
                                       CefRefPtr<CefFrame> frame,
                                       CefRefPtr<CefV8Context> context) {
  if (!IsFubukiAppFrame(frame)) {
    return;
  }
  if (rendererRouter_) {
    rendererRouter_->OnContextCreated(browser, frame, context);
  }

  auto attrs = static_cast<cef_v8_propertyattribute_t>(V8_PROPERTY_ATTRIBUTE_READONLY |
                                                       V8_PROPERTY_ATTRIBUTE_DONTDELETE);
  CefRefPtr<CefV8Value> global = context->GetGlobal();
  CefRefPtr<CefV8Value> fubuki = CefV8Value::CreateObject(nullptr, nullptr);
  fubuki->SetValue("bridgeVersion", CefV8Value::CreateString("1"), attrs);
  global->SetValue("fubukiNativeMarker", CefV8Value::CreateBool(true), attrs);
  global->SetValue("fubuki", fubuki, attrs);
}

void FubukiRenderApp::OnContextReleased(CefRefPtr<CefBrowser> browser,
                                        CefRefPtr<CefFrame> frame,
                                        CefRefPtr<CefV8Context> context) {
  if (rendererRouter_ && IsFubukiAppFrame(frame)) {
    rendererRouter_->OnContextReleased(browser, frame, context);
  }
}

bool FubukiRenderApp::OnProcessMessageReceived(CefRefPtr<CefBrowser> browser,
                                               CefRefPtr<CefFrame> frame,
                                               CefProcessId source_process,
                                               CefRefPtr<CefProcessMessage> message) {
  if (rendererRouter_) {
    return rendererRouter_->OnProcessMessageReceived(browser, frame, source_process, message);
  }
  return false;
}

}  // namespace fubuki
