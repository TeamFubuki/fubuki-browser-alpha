# Bridge

The UI talks to native code through `window.fubuki`.

- `window.fubuki.invoke(method, params)` remains the compatibility API.
- New UI code should prefer typed wrappers from `ui/src/bridge/fubuki.ts`.
- `BridgeMethodMap` describes common method parameters and results.
- `EventMap` describes native event names consumed by the UI.
- Frost Protocol v0 names are available during the migration. The UI calls
  `app.snapshot` first and falls back to `app.getState` when running against an
  older host.

Native Frost Protocol bridge access is intended for `fubuki://app/` only. Internal content pages
receive a different, capability-limited action channel. Native verifies the source page and action
key, performs the action through FrostEngine, and returns without page navigation. The legacy
`fubuki://settings/set` form route remains for compatibility, destructive actions are POST-only,
and direct destructive GET requests are rejected.

FrostEngine-to-host side effects use a separate versioned JSON boundary:

- FrostEngine emits `HostCommand` messages.
- Native executes host side effects and returns `HostCommandResult`.
- Native forwards CEF callbacks and OS observations as `HostEvent`.

Common methods:

- `app.snapshot`
- `app.getState`
- `commands.list`
- `commands.execute`
- `tabs.list`
- `tabs.create`
- `tabs.navigate`
- `tabs.activate`
- `tabs.close`
- `windows.list`
- `windows.create`
- `windows.close`
- `bookmarks.list`
- `history.list`
- `downloads.list`
- `settings.get`
- `settings.set`

Keep raw method strings localized to bridge wrappers when adding new UI features.
