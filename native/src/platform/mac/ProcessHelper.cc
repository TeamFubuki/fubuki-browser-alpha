#include "cef/FubukiRenderApp.h"
#include "include/cef_app.h"
#include "include/wrapper/cef_library_loader.h"

int main(int argc, char *argv[]) {
  CefScopedLibraryLoader libraryLoader;
  if (!libraryLoader.LoadInHelper()) {
    return 1;
  }

  CefMainArgs mainArgs(argc, argv);
  CefRefPtr<fubuki::FubukiRenderApp> app = new fubuki::FubukiRenderApp();
  return CefExecuteProcess(mainArgs, app, nullptr);
}
