import { createStore } from "solid-js/store";
import { fubuki, type BrowserState } from "../bridge/fubuki";

const initialState: BrowserState & { status: string } = {
  bridgeVersion: "1",
  activeTabId: "",
  tabs: [],
  history: [],
  bookmarks: [],
  downloads: [],
  logs: [],
  settings: {
    homepage: "https://example.com",
    downloadDirectory: "",
    searchEngine: "google",
    customSearchUrl: "https://www.google.com/search?q={query}",
    startupBehavior: "homepage",
    theme: "light",
    language: "en",
    newTabBackgroundMode: "unsplash",
    newTabBackgroundColor: "#f8fafd",
    newTabBackgroundUrl: ""
  },
  profilePath: "",
  status: "Starting"
};

export const [browserState, setBrowserState] = createStore(initialState);

let pendingRefresh: Promise<void> | undefined;
let pendingStatus = "Ready";

export async function refreshState(status = "Ready") {
  pendingStatus = status;
  if (pendingRefresh) {
    return pendingRefresh;
  }
  pendingRefresh = Promise.resolve().then(async () => {
    const statusToApply = pendingStatus;
    const state = await fubuki.invoke<BrowserState>("app.getState");
    setBrowserState({ ...state, status: statusToApply });
  }).finally(() => {
    pendingRefresh = undefined;
  });
  return pendingRefresh;
}

export function bindNativeEvents() {
  const refreshEvents = [
    "tabs.created",
    "tabs.updated",
    "tabs.closed",
    "tabs.activated",
    "navigation.started",
    "navigation.finished",
    "navigation.failed",
    "downloads.updated",
    "app.stateChanged"
  ];

  const disposers = refreshEvents.map((eventName) =>
    fubuki.on(eventName, () => {
      void refreshState(eventName);
    })
  );

  void refreshState("Ready");
  return () => disposers.forEach((dispose) => dispose());
}
