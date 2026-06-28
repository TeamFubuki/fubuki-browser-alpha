# Bridge API

Bridge version: `1`

The UI calls native code through:

```ts
window.fubuki.invoke(method, params)
```

The native bridge only accepts calls from `fubuki://app/`.

## Methods

- `app.getState`
- `tabs.create({ url?: string, active?: boolean })`
- `tabs.activate({ tabId: string })`
- `tabs.close({ tabId: string })`
- `tabs.navigate({ tabId: string, input: string })`
- `tabs.reload({ tabId: string })`
- `tabs.stop({ tabId: string })`
- `tabs.goBack({ tabId: string })`
- `tabs.goForward({ tabId: string })`
- `bookmarks.addActive()`
- `bookmarks.remove({ url: string })`
- `settings.set({ key: "homepage" | "downloadDirectory", value: string })`
- `commands.execute({ id: string, args?: object })`

## Events

- `tabs.created`
- `tabs.updated`
- `tabs.closed`
- `tabs.activated`
- `navigation.started`
- `navigation.finished`
- `navigation.failed`
- `downloads.updated`
- `app.stateChanged`

The UI listens for native events using:

```ts
window.fubuki.on("app.stateChanged", () => {
  void window.fubuki.invoke("app.getState");
});
```
