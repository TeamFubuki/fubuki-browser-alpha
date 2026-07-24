# Bridge API

Bridge version: `1`

Frost Protocol compatibility: `0`

The UI calls native code through:

```ts
window.fubuki.invoke(method, params)
```

The native bridge only accepts calls from `fubuki://app/`.

## Request validation

Every request must have exactly these top-level fields: `version: 0`,
`bridgeVersion: "1"`, `method`, and an optional object `params`. Unknown
top-level fields and unknown parameter fields are rejected with a 400 response.
The error identifies the method and field, but never echoes the submitted value.

Required string fields must be non-empty. Identifiers (`tabId`, `windowId`,
setting keys, and command IDs) are limited to 256 characters; URLs and tab
inputs to 8,192; file paths to 4,096; general text to 4,096; and setting values
to 16,384 characters. `ui.setSidebarWidth.width` is 160–800, overlay dimensions
are 100–2,000, and `tabs.move.toIndex` is 0–10,000. Enumerated values are
strictly checked for history range, data-clear target, and permission value.

## Methods

- `app.snapshot`
- `app.getState`
- `app.openDevTools`
- `tabs.list()`
- `tabs.create({ url?: string, active?: boolean })`
- `tabs.activate({ tabId: string })`
- `tabs.close({ tabId: string })`
- `tabs.pin({ tabId: string, pinned: boolean })`
- `tabs.duplicate({ tabId: string })`
- `tabs.reopenClosed()`
- `tabs.closeOther({ tabId: string })`
- `tabs.closeToRight({ tabId: string })`
- `tabs.move({ tabId: string, toIndex: number })`
- `tabs.moveToNewWindow({ tabId: string })`
- `tabs.navigate({ tabId: string, input: string })`
- `tabs.reload({ tabId: string })`
- `tabs.stop({ tabId: string })`
- `tabs.goBack({ tabId: string })`
- `tabs.goForward({ tabId: string })`
- `tabs.home()`
- `windows.list()`
- `windows.create()`
- `windows.createPrivate()`
- `windows.close()`
- `windows.reopenClosed()`
- `page.find({ query: string, forward?: boolean })`
- `page.stopFinding({ clear?: boolean })`
- `page.zoomIn()`
- `page.zoomOut()`
- `page.zoomReset()`
- `page.print()`
- `page.viewSource()`
- `bookmarks.addActive()`
- `bookmarks.list()`
- `bookmarks.save({ title: string, url: string, faviconUrl?: string })`
- `bookmarks.remove({ url: string })`
- `history.list()`
- `history.remove({ url: string })`
- `history.clearRange({ range: "lastHour" | "today" | "all" })`
- `downloads.list()`
- `downloads.remove({ url?: string, path?: string })`
- `downloads.open({ path: string })`
- `downloads.reveal({ path: string })`
- `data.clear({ target: "history" | "cookies" | "cache" | "downloads" | "siteData" | "all" })`
- `settings.get({ key: string })`
- `settings.set({ key: string, value: string })`
- `settings.reset({ key: string })`
- `permissions.set({ origin: string, permission: string, value: "ask" | "allow" | "deny" })`
- `commands.execute({ id: string, args?: object })`
- `commands.list()`
- `frost.coreSnapshot()` diagnostic endpoint for native-to-Rust bridge verification

## Events

- `tab.created`
- `tab.updated`
- `tab.closed`
- `tab.activated`
- `window.created`
- `window.closed`
- `window.focused`
- `tabs.created`
- `tabs.updated`
- `tabs.closed`
- `tabs.activated`
- `navigation.started`
- `navigation.finished`
- `navigation.failed`
- `bookmark.changed`
- `history.changed`
- `download.changed`
- `setting.changed`
- `permission.changed`
- `app.stateChanged`

The UI listens for native events using:

```ts
window.fubuki.on("app.stateChanged", () => {
  void window.fubuki.invoke("app.getState");
});
```
