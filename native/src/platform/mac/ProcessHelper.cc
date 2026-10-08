#include "cef/FubukiRenderApp.h"
#include "include/cef_app.h"
#include "include/cef_sandbox_mac.h"
#include "include/wrapper/cef_library_loader.h"

#if !defined(CEF_USE_SANDBOX)
#error "Fubuki helpers require the CEF process sandbox"
#endif

int main(int argc, char *argv[]) {
  // CEF 138+ loads libcef_sandbox.dylib before the main framework. Keep the
  // context alive until CefExecuteProcess returns; initialization must fail shut.
  CefScopedSandboxContext sandboxContext;
  if (!sandboxContext.Initialize(argc, argv)) {
    return 1;
  }

  CefScopedLibraryLoader libraryLoader;
  if (!libraryLoader.LoadInHelper()) {
    return 1;
  }

  CefMainArgs mainArgs(argc, argv);
  CefRefPtr<fubuki::FubukiRenderApp> app = new fubuki::FubukiRenderApp();
  return CefExecuteProcess(mainArgs, app, nullptr);
}
