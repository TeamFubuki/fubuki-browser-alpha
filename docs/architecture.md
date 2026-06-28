# Architecture

Fubuki Browser Alpha is split into four main layers.

## Native Core

`native/src/browser` owns the macOS window and CEF browser instances. The UI browser loads `fubuki://app/`; each tab owns a separate CEF browser for real page content. Web pages are never embedded with iframes.

`TabManager` owns tab state:

```cpp
Tab {
  id, title, url, isLoading, canGoBack, canGoForward, isActive
}
```

CEF callbacks update this state through `BrowserWindow`.

## Bridge

`NativeBridge` is the only native entry point for the SolidJS UI. Calls arrive as JSON through CEF message routing and are accepted only from `fubuki://app/`. Normal HTTPS pages do not receive the `window.fubuki` wrapper.

## Commands

`CommandRegistry` maps command IDs to handlers. UI controls may call specific bridge methods, while future shortcuts, command palettes, menus, extensions, and macros can call the same actions through `commands.execute`.

## Events

`EventBus` decouples CEF callbacks, tab state changes, and UI notifications. The MVP uses simple observer callbacks for tab and navigation events.
