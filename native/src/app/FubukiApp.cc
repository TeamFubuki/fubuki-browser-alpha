#include "app/FubukiApp.h"

#include "cef/FubukiCefApp.h"
#include "include/cef_app.h"
#include "include/wrapper/cef_library_loader.h"

#include <cstdlib>
#include <filesystem>

namespace fubuki {

void InitializeMacApplication();

int RunFubukiApplication(int argc, char *argv[]) {
  CefScopedLibraryLoader libraryLoader;
  if (!libraryLoader.LoadInMain()) {
    return 1;
  }

  CefMainArgs mainArgs(argc, argv);
  CefRefPtr<FubukiCefApp> app = new FubukiCefApp(FUBUKI_UI_DIST);

  const int exitCode = CefExecuteProcess(mainArgs, app, nullptr);
  if (exitCode >= 0) {
    return exitCode;
  }

  CefSettings settings;
  settings.no_sandbox = true;
  settings.persist_session_cookies = true;
  settings.background_color = CefColorSetARGB(0, 255, 255, 255);
  const char *home = std::getenv("HOME");
  const auto basePath =
      home ? std::filesystem::path(home) /
                 "Library/Application Support/Fubuki Browser Alpha"
           : std::filesystem::temp_directory_path() / "Fubuki Browser Alpha";
  std::filesystem::create_directories(basePath);
  const auto cachePath = basePath / "CEFProfile";
  const auto logPath = basePath / "cef.log";
  CefString(&settings.root_cache_path).FromString(basePath.string());
  CefString(&settings.cache_path).FromString(cachePath.string());
  CefString(&settings.log_file).FromString(logPath.string());

  InitializeMacApplication();

  if (!CefInitialize(mainArgs, settings, app, nullptr)) {
    return 1;
  }

  CefRunMessageLoop();
  CefShutdown();
  return 0;
}

}  // namespace fubuki
