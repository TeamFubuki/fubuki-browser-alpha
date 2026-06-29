export type FubukiBridgeMethod =
  | "app.getState"
  | "tabs.create"
  | "tabs.activate"
  | "tabs.close"
  | "tabs.navigate"
  | "tabs.reload"
  | "tabs.stop"
  | "tabs.goBack"
  | "tabs.goForward"
  | "bookmarks.addActive"
  | "bookmarks.remove"
  | "settings.set"
  | "ui.setOverlayActive"
  | "commands.execute";

export type FubukiBridgeEvent =
  | "tabs.created"
  | "tabs.updated"
  | "tabs.closed"
  | "tabs.activated"
  | "navigation.started"
  | "navigation.finished"
  | "navigation.failed"
  | "downloads.updated"
  | "app.stateChanged";
