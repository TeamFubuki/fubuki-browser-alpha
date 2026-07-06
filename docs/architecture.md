# Architecture

Fubuki Browser Alpha is a macOS-first browser shell composed of two independently versioned components:

- **Fubuki Browser UI** — the SolidJS frontend
- **FrostEngine** — the Rust-based browser state and operations core

The product ships as a single binary, but UI and engine are developed and versioned separately. The engine exposes a protocol; any client that implements it can control the browser.

## Component Versions

| Component | Location | Current |
|---|---|---|
| Fubuki Browser UI | `ui/` | `v0.1.0` |
| FrostEngine | `crates/` | `v0.1.0` |

Each component has its own `Cargo.toml` / `package.json` version and changelog. Protocol compatibility is tracked separately (see Frost Protocol below).

## Layered Structure

```
Fubuki Browser UI (SolidJS)
    ↓  Frost Protocol (JSON-RPC over CEF message router)
FrostEngine Core (Rust)
    ↓  EngineAdapter trait
CEF / macOS Host (C++)
```

- **FrostEngine Core** owns all browser state (tabs, windows, settings, session). It is implemented in Rust and has no dependency on CEF or macOS.
- **CEF / macOS Host** is a thin rendering and input layer. It creates NSWindows, manages CEF browser instances, and forwards CEF callbacks to the engine. It holds no browser state of its own.
- **Frost Protocol** is a typed request/response/event protocol. The UI sends `Request` messages and receives `Response` messages. State changes are pushed as `Event` messages.
- **Fubuki Browser UI** is the SolidJS application loaded in the UI browser (`fubuki://app/`). It communicates exclusively through Frost Protocol.

## FrostEngine Core (`crates/frost-core`)

`BrowserCore` is the top-level entry point. It owns four services:

| Service | Responsibility |
|---|---|
| `TabService` | Tab creation, closure, activation, ordering, pinning |
| `WindowService` | Window creation, closure, focus, session snapshot |
| `SettingsService` | Settings read/write, defaults |
| `SessionService` | Session snapshot persistence and restoration |

`BrowserCore::process(Request) -> Response` handles all protocol requests. State changes emit `Event` messages through a broadcast channel.

## Frost Protocol (`crates/frost-protocol`)

Defines typed `Request`, `Response`, `Event`, and state schemas (`TabState`, `WindowState`, `AppState`).

### API (v0)

| Request | Response |
|---|---|
| `app.snapshot` | Full `AppState` |
| `tabs.list` | `Vec<TabState>` |
| `tabs.create { url?, active }` | `Ok(bool)` |
| `tabs.activate { tab_id }` | `Ok(bool)` |
| `tabs.close { tab_id }` | `Ok(bool)` |
| `tabs.navigate { tab_id, input }` | `Ok(bool)` |
| `tabs.reload { tab_id }` | `Ok(bool)` |
| `tabs.goBack { tab_id }` | `Ok(bool)` |
| `tabs.goForward { tab_id }` | `Ok(bool)` |
| `windows.list` | `Vec<WindowState>` |
| `windows.create` | `Ok(bool)` |
| `windows.close` | `Ok(bool)` |
| `settings.get { key }` | `Ok(String)` |
| `settings.set { key, value }` | `Ok(bool)` |

### Events (differential)

| Event | Payload |
|---|---|
| `tab.created` | `TabState` |
| `tab.updated` | `TabPatch` |
| `tab.closed` | `{ tab_id }` |
| `tab.activated` | `{ tab_id }` |
| `window.created` | `WindowState` |
| `window.closed` | `{ window_id }` |
| `setting.changed` | `{ key, value }` |

## State Synchronization

1. On startup the UI calls `app.snapshot` to receive the full `AppState`.
2. After that, differential `Event` messages update the UI store incrementally.
3. Full resync via `app.snapshot` only on state inconsistency or recovery.

## EngineAdapter (`crates/frost-engine-api`)

Defines the boundary between FrostEngine Core and the host:

- `EngineAdapter` — open, navigate, reload, close pages; create and close windows.
- `PageAdapter` — receive page-level callbacks (title change, URL change, loading state, favicon).
- `WindowHost` — manage NSWindow lifecycle.

The CEF/macOS Host implements these traits. Future hosts (e.g., a headless server, a different browser engine) would implement the same traits.

## Persistence (`crates/frost-store`)

SQLite-based persistence with a repository pattern:

- `SettingsRepository` — key/value settings
- `HistoryRepository` — browsing history
- `BookmarkRepository` — bookmarks
- `DownloadRepository` — download records
- `SessionRepository` — window/tab snapshots for session restore

Migrations are versioned and applied on startup.

## CEF / macOS Host (`native/macos-cef-host`)

The host is responsible for:

- Creating and managing `NSWindow` instances
- Creating CEF browser instances for the UI and each tab
- Handling the `fubuki://` scheme
- Forwarding CEF callbacks (title, URL, loading state, favicon, downloads) to the engine via `PageAdapter`
- Receiving `EngineAdapter` calls from the engine to perform CEF operations

The host holds no browser state. It is a pure I/O layer.

## Legacy (pre-FrostEngine)

The current codebase (`native/src/browser/`, `native/src/bridge/`) will be incrementally migrated:

- `BrowserWindow` → thin CEF host, state management removed
- `NativeBridge` → thin Frost Protocol entry point
- `TabManager` → replaced by `TabService`
- `BrowserDataStore` → replaced by `frost-store`
- `BrowserAppController` → window management moves to `WindowService`
- `CommandRegistry` → command schema defined in `frost-protocol`

See `docs/frost-engine-plan.md` for the detailed migration plan.
