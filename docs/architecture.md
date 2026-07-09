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
    ↓  HostCommand / HostEvent (versioned JSON)
CEF / macOS Host (C++)
```

- **FrostEngine Core** owns all browser state (tabs, windows, settings, session). It is implemented in Rust and has no dependency on CEF or macOS.
- **CEF / macOS Host** is a thin rendering and input layer. It creates NSWindows, manages CEF browser instances, executes `HostCommand`s, and forwards CEF callbacks as `HostEvent`s. It should not own logical browser state.
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

## Host Boundary (`crates/frost-protocol`, `crates/frost-engine-api`)

Defines the boundary between FrostEngine Core and the host:

- `HostCommand` — versioned JSON commands emitted by FrostEngine for host side effects such as page creation, navigation, reload, stop, and window lifecycle.
- `HostEvent` — versioned JSON events sent back by the host for page title, URL, favicon, loading state, navigation state, downloads, permissions, and window focus/closure.
- `HostCommandResult` — completion/failure status for a previously emitted host command.
- `EngineAdapter` — Rust-side abstraction used by `BrowserCore`; the production FFI adapter serializes calls into `HostCommand`s.

Future hosts (e.g., a headless server, a different browser engine) should implement the JSON host boundary rather than reaching into `BrowserCore` internals.

## External Automation Boundary

External automation and MCP-style clients connect at FrostEngine's command layer through `ExternalCommand` and declared capabilities:

- `read_state`
- `tab_control`
- `navigation`
- `bookmarks`
- `history`
- `downloads`
- `debug`

Destructive external commands must pass capability checks and produce audit events before they are routed to services or host commands.

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
- Executing `HostCommand`s received from FrostEngine
- Forwarding CEF callbacks (title, URL, loading state, favicon, downloads) to the engine as `HostEvent`s
- Returning `HostCommandResult` for host side effects

The host holds no browser state. It is a pure I/O layer.

Destructive internal-page actions must not execute from URL GET navigation. Internal pages use POST actions for destructive operations, and the host rejects destructive `fubuki://settings/set?...` GET requests.

## Migration Status

The legacy native state ownership has been migrated to FrostEngine:

| Component | Status |
|---|---|
| `BrowserDataStore` (SQLite + CefListValue caches) | **Removed.** Replaced by `frost-store` via the `FrostStore` FFI wrapper (`native/src/browser/FrostStore.*`). Settings, logs, bookmarks, history, downloads, permissions, and session all live in the engine-owned SQLite database. |
| `NativeBridge` `host.syncSnapshot` reverse sync | **Removed.** FrostEngine is the single source of truth; the host no longer pushes a synthesized snapshot back into the engine. |
| Legacy bridge methods (`app.getState`, `frost.coreSnapshot`) | **Removed.** UI and native both go through Frost Protocol request/response only. |
| Destructive `fubuki://settings/set?...` GET | **Rejected.** The scheme handler returns HTTP 403 for any GET-style `settings/set` navigation; settings changes only fire from POST form submissions. |
| `TabManager` | Retained as a CEF browser-instance manager (per the host's responsibility for CEF lifecycle). Logical tab state (existence, active, ordering) is owned by `TabService`; the host reflects CEF callbacks into the engine via `HostEvent`s and reads state back through `app.snapshot`. |
| `BrowserAppController` | Still owns NSWindow lifecycle on the host; window logical state is mirrored into `WindowService` via `HostCommand`/`HostEvent`. |
| `CommandRegistry` | Command schema is defined in `frost-protocol`; native registry remains a thin dispatcher. |
| External / MCP boundary | Implemented in `frost-core::external_router` with capability gating, audit events, and a rate limiter. Native reaches it through `FrostBridge::GrantExternal` / `ProcessExternalJson`. |

See `docs/frost-engine-plan.md` for the detailed migration plan.
