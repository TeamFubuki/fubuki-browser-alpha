# Events

Native state changes are published through the event bus and forwarded to the UI. FrostEngine
remains the source of truth; the UI applies the differential events below and only requests a
full `app.snapshot` when it starts, receives the compatibility `app.stateChanged` event, or
detects an inconsistent tab/window event.

`ui/src/stores/browserStore.ts` handles events as follows:

- Tab and window events (`tab.*`, `window.*`) are applied directly by the event reducer.
- `setting.changed` patches the changed setting key directly.
- `bookmark.changed` and `history.changed` refresh only their respective list endpoints, with
  concurrent requests coalesced.
- `download.changed` and `downloads.updated` refresh only `downloads.list`, throttled to one
  request per 100 ms while progress is active.
- `permission.changed` is accepted but deferred until the next full snapshot because the main
  UI does not currently render permission records.
- `app.stateChanged` requests a full `app.snapshot` for compatibility and recovery.
