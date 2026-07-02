# Architecture

Fubuki Browser Alpha is split into four main layers.

## Native Core

`native/src/browser` owns the macOS windows and CEF browser instances. `BrowserAppController` coordinates multiple `BrowserWindow` instances, each with its own `TabManager`, active tab, and UI browser. The UI browser loads `fubuki://app/`; each tab owns a separate CEF browser for real page content. Web pages are never embedded with iframes.

`TabManager` owns tab state:

```cpp
Tab {
  id, title, url, faviconUrl, errorText, zoomLevel,
  isLoading, canGoBack, canGoForward, isActive, isPinned
}
```

CEF callbacks update this state through `BrowserWindow`.

`BrowserDataStore` stores history, bookmarks, downloads, settings, permissions, logs, and the session snapshot in SQLite under the user profile directory. Normal windows participate in session persistence. Private windows use an off-the-record CEF request context and skip normal history, download-history, log, and session writes.

## Bridge

`NativeBridge` is the only native entry point for the SolidJS UI. Calls arrive as JSON through CEF message routing and are accepted only from `fubuki://app/`. Normal HTTPS pages do not receive the `window.fubuki` wrapper.

## Commands

`CommandRegistry` maps command IDs to handlers. UI controls may call specific bridge methods, while future shortcuts, command palettes, menus, extensions, and macros can call the same actions through `commands.execute`.

## Events

`EventBus` decouples CEF callbacks, tab/window state changes, data-store changes, and UI notifications. It publishes stable structured event names for window, tab, navigation, bookmark, history, download, setting, and permission changes, and keeps a recent-event buffer for `fubuki://debug/`.
