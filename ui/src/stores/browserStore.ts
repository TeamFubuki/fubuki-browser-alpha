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
    searchEngine: "duckduckgo",
    startupBehavior: "homepage"
  },
  profilePath: "",
  status: "Starting"
};

export const [browserState, setBrowserState] = createStore(initialState);

export async function refreshState(status = "Ready") {
  const state = await fubuki.invoke<BrowserState>("app.getState");
  setBrowserState({ ...state, status });
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
