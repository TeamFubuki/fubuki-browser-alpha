# Events

Native state changes are published through the event bus and forwarded to the UI.

The UI currently performs a coalesced full `app.getState` refresh for these events in
`ui/src/stores/browserStore.ts`:

- `tabs.created`
- `tabs.updated`
- `tabs.closed`
- `tabs.activated`
- `navigation.started`
- `navigation.finished`
- `navigation.failed`
- `downloads.updated`
- `download.changed`
- `bookmark.changed`
- `history.changed`
- `setting.changed`
- `permission.changed`
- `window.created`
- `window.closed`
- `window.focused`
- `app.stateChanged`

This is simple and safe, but not always minimal. Future improvements should narrow high-volume
events first, especially tab navigation and download progress.
