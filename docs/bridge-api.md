# Bridge API

Bridge version: `1`

Frost Protocol compatibility: `0`

The UI calls native code through:

```ts
window.fubuki.invoke(method, params)
```

The native bridge only accepts calls from `fubuki://app/`.

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
