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

## Implemented Essentials

- Native macOS CEF app windows with independent tab managers per window
- Separate CEF browser for `fubuki://app/` UI and separate CEF browsers for web tab contents
- New window, private window, close window, and move tab to a new window
- Startup behavior setting: new tab, restore previous session, or home page
- Session restore for normal windows, tabs, active tabs, pinned state, window frame, and sidebar state
- Private windows use an off-the-record CEF request context and skip normal history, downloads history, logs, and session writes
- Tab create, activate, close, pin/unpin, duplicate, reopen closed, close other tabs, close tabs to the right, drag reorder, tab search, and last-tab replacement
- Back, forward, reload, stop, home, and omnibox navigation
- URL normalization for full URLs, host names such as `github.com`, and search queries
- Page title, favicon, loading, error, zoom, and basic security state updates
- Page find, zoom in/out/reset, print, view source, and DevTools actions
- Persistent CEF profile for cookies, LocalStorage, and IndexedDB
- Download handling with progress records, open, reveal in Finder, remove, and clear list actions
- SQLite-backed local history, bookmarks, downloads, settings, permissions, and debug logs
- Built-in history, bookmarks, downloads, settings, new tab, and debug internal pages
- History search, grouping by date, individual deletion, and clear last hour/today/all time
- Settings sections for General, Appearance, Tabs, Windows, Search, Privacy, Downloads, Shortcuts, Developer, and Experimental
- Error page for failed navigations
- Versioned JSON bridge exposed only to `fubuki://app/`
- Command registry for browser actions plus `commands.list` for UI/future plugin API discovery
- Event bus for window, tab, navigation, bookmark, history, download, setting, and permission changes
- `fubuki://debug/` with bridge version, profile path, windows/tabs, commands, recent events, and logs

## Keyboard Shortcuts

- `Cmd+L`: focus the URL/search bar
- `Cmd+N`: new window
- `Cmd+Shift+N`: new private window
- `Cmd+T`: new tab
- `Cmd+W`: close tab
- `Cmd+Shift+W`: close window
- `Cmd+Shift+T`: reopen closed tab
- `Cmd+R`: reload
- `Cmd+F`: find in page
- `Cmd+[` / `Cmd+]`: back / forward
- `Cmd+D`: bookmark active tab
- `Cmd+,`: settings
- `Cmd+Plus` / `Cmd+Minus` / `Cmd+0`: zoom in / zoom out / reset zoom

## Profile Data

Runtime profile data is stored under:

```text
~/Library/Application Support/Fubuki Browser Alpha/
```

This includes the CEF profile for cookies, LocalStorage, IndexedDB, cache data, plus `fubuki.sqlite3` for history, bookmarks, downloads, settings, permissions, session snapshots, and debug logs.

## Known Limitations

- CEF binaries, codesigning, notarization, and update packaging are not included.
- Browser layout uses native child views and a compact web UI toolbar/sidebar.
- Password manager and sync are not implemented.
- Chrome extension compatibility is not implemented.
- Cache/site-data clearing depends on CEF request-context support and is intentionally conservative.
- Bookmark folders and browser-compatible import/export are not complete yet.
