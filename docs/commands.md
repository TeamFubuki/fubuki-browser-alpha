# Commands

Browser actions are registered in `BrowserWindow::RegisterCommands()`.

The UI command palette uses `commands.list` and executes command IDs through
`commands.execute`. UI-only commands, such as Quiet Mode, may be added locally when they do not
need native state.

Important command IDs:

- `tabs.create`
- `tabs.close`
- `tabs.reopenClosed`
- `app.openSettings`
- `app.openHistory`
- `app.openBookmarks`
- `app.openDownloads`
- `app.toggleSidebar`
- `app.openDebug`
- `app.openDevTools`
- `page.zoomReset`

When adding a native browser action, register it with a stable ID, clear title, category, and
shortcut if one exists. Keep IDs namespaced (`tabs.*`, `app.*`, `page.*`, `bookmarks.*`).
