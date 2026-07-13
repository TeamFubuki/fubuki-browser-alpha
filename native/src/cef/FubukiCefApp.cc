#include "cef/FubukiCefApp.h"

#include <cstdlib>
#include <filesystem>

#include "cef/FubukiSchemeHandler.h"
#include "include/cef_scheme.h"
#include "include/wrapper/cef_helpers.h"

namespace fubuki {

namespace {

bool IsFubukiAppFrame(CefRefPtr<CefFrame> frame) {
  return frame && frame->GetURL().ToString().rfind("fubuki://app/", 0) == 0;
}

void InstallWebAuthnGuard(CefRefPtr<CefFrame> frame) {
  if (!frame || IsFubukiAppFrame(frame)) {
    return;
  }

  frame->ExecuteJavaScript(
      R"JS(
(() => {
  try {
    const credentials = navigator.credentials;
    if (!credentials || credentials.__fubukiWebAuthnGuard) return;
    const rejectPasskey = () => Promise.reject(new DOMException("Passkeys are not available in this build.", "NotAllowedError"));
    const nativeCreate = credentials.create ? credentials.create.bind(credentials) : undefined;
    const nativeGet = credentials.get ? credentials.get.bind(credentials) : undefined;
    Object.defineProperty(credentials, "__fubukiWebAuthnGuard", { value: true });
    if (nativeCreate) {
      Object.defineProperty(credentials, "create", { configurable: true, value: (options) => options && options.publicKey ? rejectPasskey() : nativeCreate(options) });
    }
    if (nativeGet) {
      Object.defineProperty(credentials, "get", { configurable: true, value: (options) => options && options.publicKey ? rejectPasskey() : nativeGet(options) });
    }
  } catch (error) {
    console.error("[Fubuki] Failed to install WebAuthn guard", error);
  }
})();
)JS",
      frame->GetURL(), 0);
}

}  // namespace

FubukiCefApp::FubukiCefApp(std::string uiDistPath)
    : uiDistPath_(std::move(uiDistPath)) {}

void FubukiCefApp::OnRegisterCustomSchemes(
    CefRawPtr<CefSchemeRegistrar> registrar) {
  registrar->AddCustomScheme("fubuki", CEF_SCHEME_OPTION_STANDARD |
                                           CEF_SCHEME_OPTION_SECURE |
                                           CEF_SCHEME_OPTION_CORS_ENABLED |
                                           CEF_SCHEME_OPTION_FETCH_ENABLED);
}

void FubukiCefApp::OnContextInitialized() {
  CEF_REQUIRE_UI_THREAD();
  CefRegisterSchemeHandlerFactory("fubuki", "app",
                                  new FubukiSchemeHandlerFactory(uiDistPath_));
  CefRegisterSchemeHandlerFactory("fubuki", "newtab",
                                  new FubukiSchemeHandlerFactory(uiDistPath_));
  CefRegisterSchemeHandlerFactory("fubuki", "settings",
                                  new FubukiSchemeHandlerFactory(uiDistPath_));
  CefRegisterSchemeHandlerFactory("fubuki", "bookmarks",
                                  new FubukiSchemeHandlerFactory(uiDistPath_));
  CefRegisterSchemeHandlerFactory("fubuki", "downloads",
                                  new FubukiSchemeHandlerFactory(uiDistPath_));
  CefRegisterSchemeHandlerFactory("fubuki", "history",
                                  new FubukiSchemeHandlerFactory(uiDistPath_));
  CefRegisterSchemeHandlerFactory("fubuki", "debug",
                                  new FubukiSchemeHandlerFactory(uiDistPath_));

  const char *home = std::getenv("HOME");
  const auto requestedProfilePath =
      home ? std::filesystem::path(home) /
                 "Library/Application Support/Fubuki Browser Alpha"
           : std::filesystem::temp_directory_path() / "Fubuki Browser Alpha";
  std::filesystem::create_directories(requestedProfilePath);
  std::error_code profileError;
  const auto profilePath =
      std::filesystem::canonical(requestedProfilePath, profileError);
  if (profileError) {
    LOG(FATAL) << "Failed to canonicalize the browser profile path";
    return;
  }
  browserApp_ =
      std::make_unique<BrowserAppController>(profilePath, uiDistPath_);
  SetBrowserAppController(browserApp_.get());
  browserApp_->Start();
}

void FubukiCefApp::OnWebKitInitialized() {
  CefMessageRouterConfig config;
  config.js_query_function = "cefQuery";
  config.js_cancel_function = "cefQueryCancel";
  rendererRouter_ = CefMessageRouterRendererSide::Create(config);
}

void FubukiCefApp::OnContextCreated(CefRefPtr<CefBrowser> browser,
                                    CefRefPtr<CefFrame> frame,
                                    CefRefPtr<CefV8Context> context) {
  if (!IsFubukiAppFrame(frame)) {
    InstallWebAuthnGuard(frame);
    return;
  }
  if (rendererRouter_) {
    rendererRouter_->OnContextCreated(browser, frame, context);
  }

  auto attrs = static_cast<cef_v8_propertyattribute_t>(
      V8_PROPERTY_ATTRIBUTE_READONLY | V8_PROPERTY_ATTRIBUTE_DONTDELETE);
  CefRefPtr<CefV8Value> global = context->GetGlobal();
  global->SetValue("fubukiNativeMarker", CefV8Value::CreateBool(true), attrs);
}

void FubukiCefApp::OnContextReleased(CefRefPtr<CefBrowser> browser,
                                     CefRefPtr<CefFrame> frame,
                                     CefRefPtr<CefV8Context> context) {
  if (rendererRouter_) {
    rendererRouter_->OnContextReleased(browser, frame, context);
  }
}

bool FubukiCefApp::OnProcessMessageReceived(
    CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame,
    CefProcessId source_process, CefRefPtr<CefProcessMessage> message) {
  if (rendererRouter_) {
    return rendererRouter_->OnProcessMessageReceived(browser, frame,
                                                     source_process, message);
  }
  return false;
}

}  // namespace fubuki
