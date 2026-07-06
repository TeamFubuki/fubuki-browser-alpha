import { createStore } from "solid-js/store";
import {
  invokeBridge,
  onBridgeEvent,
  type BrowserState,
  type EventMap,
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
let refreshCounter = 0;
let lastStatus = "Ready";

export async function refreshState(status = "Ready") {
  lastStatus = status;
  if (pendingRefresh) {
    return pendingRefresh;
  }
  const myCounter = ++refreshCounter;
  const statusAtStart = lastStatus;
  pendingRefresh = invokeBridge("app.getState")
    .then((state) => {
      // Only apply if no newer refresh has started
      if (myCounter === refreshCounter) {
        setBrowserState({ ...state, status: statusAtStart });
      }
    })
    .catch((error) => {
      console.error("[Fubuki] Failed to refresh state:", error);
      setBrowserState({ status: "Error" });
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

export function currentLanguage(): string {
  return browserState.settings.language;
}

export async function toggleBookmark(): Promise<void> {
  const tab = activeTab();
  if (!tab?.url || tab.url.startsWith("fubuki://") || tab.url.startsWith("data:"))
    return;
  try {
    if (isTabBookmarked(tab.url)) {
      await invokeBridge("bookmarks.remove", { url: tab.url });
    } else {
      await invokeBridge("bookmarks.save", {
        title: tab.title || tab.url,
        url: tab.url,
        faviconUrl: tab.faviconUrl || "",
      });
    }
    await refreshState("bookmarks.changed");
  } catch (error) {
    console.error("[Fubuki] Failed to toggle bookmark:", error);
  }
}

export function toggleSidebar(): void {
  const next =
    browserState.settings.sidebarVisible === "hide" ? "show" : "hide";
  void invokeBridge("settings.set", { key: "sidebarVisible", value: next })
    .then(() => refreshState("settings.saved"))
    .catch((error) => console.error("[Fubuki] Failed to toggle sidebar:", error));
}

export function navigateInternal(url: string): void {
  const tab = activeTab();
  const promise = tab
    ? invokeBridge("tabs.navigate", { tabId: tab.id, input: url })
    : invokeBridge("tabs.create", { url, active: true });
  void promise.catch((error) => console.error("[Fubuki] Failed to navigate:", error));
}

export function bindNativeEvents() {
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
