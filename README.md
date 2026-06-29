# Fubuki Browser Alpha

Fubuki Browser Alpha is a macOS-first browser shell built on C++20, CEF, and a SolidJS browser UI. The MVP keeps the native core, tab state, command registry, event bus, and UI bridge separated so future browser features can be added without wiring UI controls directly to CEF internals.

## Requirements

- macOS 12 or newer
- Apple Silicon CEF binary distribution matching your machine
- CMake 3.21+
- Xcode command line tools
- Node.js 20+
- pnpm 11+

Electron, Tauri, WKWebView, and WebView2 are not used.

## CEF Setup

The easiest path is:

```bash
make cef
```

This downloads the latest stable standard CEF macOS build from the CEF Automated Builds CDN into `third_party/cef/`. The script automatically selects `macosarm64` on Apple Silicon and `macosx64` on Intel.

You can also manually download a macOS CEF binary distribution from the official CEF builds site and either:

- unpack it into `third_party/cef/`, or
- pass `-DCEF_ROOT=/path/to/cef_binary` when configuring CMake.

The repository intentionally does not vendor CEF binaries.

## Build UI

```bash
cd ui
pnpm install
cd ..
make ui
```

The native app serves the generated `ui/dist` files at `fubuki://app/`.

## Build Native App

```bash
make native
```

## One-command Build and Run

```bash
make bootstrap
make build
make run
```

Useful targets:

```bash
make help
make cef
make ui
make configure
make native
make build
make run
make clean
```

Advanced manual native build:

```bash
cd native
cmake -S . -B build -DCEF_ROOT=/path/to/cef_binary -DCMAKE_BUILD_TYPE=Release
cmake --build build --config Release
open "build/Fubuki Browser Alpha.app"
```

If your generator uses configuration subdirectories, the app may be at `build/Release/Fubuki Browser Alpha.app`.

## MVP Features

- Native macOS CEF app window
- Separate CEF browser for `fubuki://app/` UI and separate CEF browsers for web tab contents
- Initial `https://example.com` tab
- Tab create, activate, close, and last-tab replacement
- Back, forward, reload, stop, and omnibox navigation
- URL normalization for full URLs, host names such as `github.com`, and search queries
- Page title and favicon updates
- Persistent CEF profile for cookies, LocalStorage, and IndexedDB
- Download handling with saved download records
- Local history, bookmarks, settings, and debug log JSON files
- Built-in history, bookmarks, downloads, settings, and logs UI
- Error page for failed navigations
- Versioned JSON bridge exposed only to `fubuki://app/`
- Command Registry with tab commands and `app.openDevTools`
- Event Bus for tab and navigation events
- `fubuki://newtab/` internal new-tab page

## Keyboard Shortcuts

- `Cmd+L`: focus the URL/search bar
- `Cmd+R`: reload
- `Cmd+[` / `Cmd+]`: back / forward
- `Cmd+T`: new tab
- `Cmd+W`: close tab
- `Cmd+D`: bookmark active tab

## Profile Data

Runtime profile data is stored under:

```text
~/Library/Application Support/Fubuki Browser Alpha/
```

This includes the CEF profile for cookies, LocalStorage, IndexedDB, cache data, plus JSON files for history, bookmarks, downloads, settings, and debug logs.

## Known Limitations

- CEF binaries, codesigning, notarization, and update packaging are not included.
- Browser layout uses native child views and a fixed-height web UI toolbar for the MVP.
- Session restore is not implemented.
- Password manager and session restore are not implemented.
- Chrome extension compatibility is not implemented.
- The bridge API is intentionally small and rejects unknown methods.
