import { createStore } from "solid-js/store";
import {
  invokeBridge,
  onBridgeEvent,
  type BrowserState,
  type BookmarkRecord,
  type EventMap,
  type Settings,
  type Tab,
} from "../bridge/fubuki";

const initialState: BrowserState & { status: string } = {
  bridgeVersion: "1",
  windowId: "",
  isPrivate: false,
  activeTabId: "",
  tabs: [],
  windows: [],
  history: [],
  bookmarks: [],
  downloads: [],
  permissions: [],
  logs: [],
  commands: [],
  recentEvents: [],
  settings: {
    homepage: "https://example.com",
    searchEngine: "google",
    customSearchUrl: "https://www.google.com/search?q={query}",
    theme: "light",
    appearance: "system",
    sidebarVisible: "show",
    sidebarWidth: "196",
    newTabPage: "blank",
    homeUrl: "https://example.com",
    language: "system",
    defaultZoomLevel: "0",
  },
  profilePath: "",
  status: "Starting",
};

export const [browserState, setBrowserState] = createStore(initialState);

let pendingRefresh: Promise<void> | undefined;
let pendingStatus = "Ready";
let refreshCounter = 0;

export async function refreshState(status = "Ready") {
  pendingStatus = status;
  if (pendingRefresh) {
    return pendingRefresh;
  }
  const currentCounter = ++refreshCounter;
  pendingRefresh = Promise.resolve()
    .then(async () => {
      const statusToApply = pendingStatus;
      const state = await invokeBridge("app.getState");
      // Only apply if no newer refresh was requested
      if (currentCounter === refreshCounter) {
        setBrowserState({ ...state, status: statusToApply });
      }
    })
    .finally(() => {
      pendingRefresh = undefined;
    });
  return pendingRefresh;
}

export function activeTab(): Tab | undefined {
  return browserState.tabs.find((tab) => tab.id === browserState.activeTabId);
}

export function isTabBookmarked(url: string | undefined): boolean {
  if (!url) return false;
  return browserState.bookmarks.some((bookmark) => bookmark.url === url);
}

export function activeTabId(): string {
  return browserState.activeTabId;
}

export function bindNativeEvents() {
  // These native events currently trigger a coalesced full app-state refresh.
  // Keep this list typed so new events are deliberate, then narrow individual refreshes later.
  const refreshEvents: Array<keyof EventMap> = [
    "tabs.created",
    "tabs.updated",
    "tabs.closed",
    "tabs.activated",
    "navigation.started",
    "navigation.finished",
    "navigation.failed",
    "downloads.updated",
    "download.changed",
    "bookmark.changed",
    "history.changed",
    "setting.changed",
    "permission.changed",
    "window.created",
    "window.closed",
    "window.focused",
    "app.stateChanged",
  ];

  const disposers = refreshEvents.map((eventName) =>
    onBridgeEvent(eventName, () => {
      void refreshState(eventName);
    })
  );

  void refreshState("Ready");
  return () => disposers.forEach((dispose) => dispose());
}
