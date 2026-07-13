# Architecture audit baseline (2026-07-13)

This file records the pre-fix state so later green checks cannot erase evidence
of architectural defects that were not covered by tests.

## State ownership and data flow before fixes

```text
SolidJS UI
  -> per-window NativeBridge
       -> per-window FrostBridge -> per-window FrostEngine ----+
       -> BrowserWindow/TabManager mutates CEF host first       | same SQLite path
  -> BrowserAppController -> application FrostEngine ----------+
       -> FrostStore protocol facade

FubukiSchemeHandler -> direct sqlite3 -> fubuki.sqlite3
FrostEngine/FrostStore             -> frost-engine.sqlite3
```

The intended flow in `docs/architecture.md` is instead:

```text
UI request -> one application FrostEngine -> HostCommand(id)
  -> CEF/macOS side effect -> HostCommandResult(id) -> FrostEngine commit/error
```

## Document/implementation contradictions found before fixes

1. `docs/architecture.md` says there is one engine-owned source of truth, but
   `BrowserAppController` owned an engine and every `NativeBridge` constructed
   another engine using the same `frost-engine.sqlite3` path.
2. The architecture document says C++ direct SQLite access and the duplicate DB
   were removed, while `FubukiSchemeHandler.cc` opens `fubuki.sqlite3`, creates
   tables, and queries them with sqlite3 directly.
3. The architecture document says legacy `app.getState`, `frost.coreSnapshot`,
   and `host.syncSnapshot` APIs are removed. They remain in protocol, UI bridge,
   shared TypeScript bridge, tests, and migration documents.
4. The documented Engine-first command flow is not consistently implemented.
   `NativeBridge::HostBackedFrostInvoke` still performs host operations around
   protocol requests, while Engine host-command results currently have no
   pending-command registry, timeout, duplicate-result detection, or rollback.
5. The host is documented as owning no logical state, but `TabManager`, closed
   tab/window collections, session snapshots, and host-generated IDs duplicate
   Engine lifecycle state.
6. CEF sandboxing is described as a security boundary, but CMake unconditionally
   defaults `USE_SANDBOX` to `OFF` as an MVP setting.
7. UI assets are documented as application resources, but the executable is
   compiled with the source-tree absolute `ui/dist` path through
   `FUBUKI_UI_DIST`.
8. CI runs strict checks, but local `make lint-native` suppresses tool stderr and
   appends `|| true`; `make format-check` also converts a failed/missing command
   into success.
9. `docs/frost-engine-plan.md`, `docs/bridge.md`, `docs/bridge-api.md`, and
   `docs/events.md` describe mutually incompatible migration stages as current.

## Pre-fix verification results

All commands below passed on macOS before fixes, demonstrating gaps in test
coverage rather than architectural correctness.

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | pass |
| `cargo test --workspace` | pass (29 tests) |
| `pnpm run format:check` | pass |
| `pnpm run lint` | pass |
| `pnpm run build` | pass |
| `pnpm run test` | pass (92 tests) |
| `make test-native` | pass (78 tests) |

The full native application build and smoke test are tracked separately because
they require the complete CEF distribution and an interactive macOS session.

