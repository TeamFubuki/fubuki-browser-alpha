# Bridge

The UI talks to native code through `window.fubuki`.

- `window.fubuki.invoke(method, params)` remains the compatibility API.
- New UI code should prefer typed wrappers from `ui/src/bridge/fubuki.ts`.
- `BridgeMethodMap` describes common method parameters and results.
- `EventMap` describes native event names consumed by the UI.

Native bridge access is intended for `fubuki://app/` only. Internal content pages such as
`fubuki://settings/` use trusted `fubuki://settings/set?...` actions handled by native code.

Common methods:

- `app.getState`
- `commands.list`
- `commands.execute`
- `tabs.create`
- `tabs.navigate`
- `tabs.activate`
- `tabs.close`
- `settings.set`

Keep raw method strings localized to bridge wrappers when adding new UI features.
