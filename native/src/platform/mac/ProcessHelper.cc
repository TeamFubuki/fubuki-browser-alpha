#include "cef/FubukiRenderApp.h"
#include "include/cef_app.h"
#include "include/wrapper/cef_library_loader.h"

#ifdef CEF_USE_SANDBOX
#include "include/cef_sandbox_mac.h"
#endif

int main(int argc, char *argv[]) {
#ifdef CEF_USE_SANDBOX
  CefScopedSandboxContext sandboxContext;
  if (!sandboxContext.Initialize(argc, const_cast<const char **>(argv))) {
    return 1;
  }
#endif

  CefScopedLibraryLoader libraryLoader;
  if (!libraryLoader.LoadInHelper()) {
    return 1;
  }

  CefMainArgs mainArgs(argc, argv);
  CefRefPtr<fubuki::FubukiRenderApp> app = new fubuki::FubukiRenderApp();
  return CefExecuteProcess(mainArgs, app, nullptr);
}
