#pragma once

#include <memory>
#include <string>

#include "browser/BrowserWindow.h"
#include "events/EventBus.h"
#include "include/cef_app.h"
#include "include/wrapper/cef_message_router.h"

namespace fubuki {

class FubukiCefApp : public CefApp,
                     public CefBrowserProcessHandler,
                     public CefRenderProcessHandler {
 public:
  explicit FubukiCefApp(std::string uiDistPath);

  CefRefPtr<CefBrowserProcessHandler> GetBrowserProcessHandler() override { return this; }
  CefRefPtr<CefRenderProcessHandler> GetRenderProcessHandler() override { return this; }

  void OnRegisterCustomSchemes(CefRawPtr<CefSchemeRegistrar> registrar) override;
  void OnContextInitialized() override;
  void OnWebKitInitialized() override;
  void OnContextCreated(CefRefPtr<CefBrowser> browser,
                        CefRefPtr<CefFrame> frame,
                        CefRefPtr<CefV8Context> context) override;
  void OnContextReleased(CefRefPtr<CefBrowser> browser,
                         CefRefPtr<CefFrame> frame,
                         CefRefPtr<CefV8Context> context) override;
  bool OnProcessMessageReceived(CefRefPtr<CefBrowser> browser,
                                CefRefPtr<CefFrame> frame,
                                CefProcessId source_process,
                                CefRefPtr<CefProcessMessage> message) override;

 private:
  std::string uiDistPath_;
  CefRefPtr<CefMessageRouterRendererSide> rendererRouter_;
  EventBus eventBus_;
  std::unique_ptr<TabManager> tabManager_;
  std::unique_ptr<BrowserWindow> browserWindow_;

  IMPLEMENT_REFCOUNTING(FubukiCefApp);
};

}  // namespace fubuki
