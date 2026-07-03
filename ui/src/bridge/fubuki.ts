export type Tab = {
  id: string;
  title: string;
  url: string;
  faviconUrl: string;
  errorText: string;
  zoomLevel: number;
  isLoading: boolean;
  canGoBack: boolean;
  canGoForward: boolean;
  isActive: boolean;
  isPinned: boolean;
};

export type TabSnapshot = {
  title: string;
  url: string;
  faviconUrl: string;
  pinned: boolean;
  active: boolean;
};

export type HistoryRecord = {
  title: string;
  url: string;
  faviconUrl: string;
  createdAt: string;
};

export type BookmarkRecord = {
  title: string;
  url: string;
  faviconUrl: string;
  createdAt: string;
};

export type DownloadRecord = {
  url: string;
  path: string;
  state: string;
  percent: number;
  createdAt: string;
};

export type PermissionRecord = {
  origin: string;
  permission: string;
  value: string;
  createdAt: string;
};

export type LogRecord = {
  level: string;
  message: string;
  createdAt: string;
};

export type EventRecord = {
  name: string;
  windowId: string;
  tabId: string;
  message: string;
};

/** @deprecated Use specific record types instead */
export type BrowserRecord = HistoryRecord | BookmarkRecord | DownloadRecord | PermissionRecord | LogRecord | EventRecord;

export type BrowserCommand = {
  id: string;
  title: string;
  category: string;
  shortcut: string;
};

export type WindowSnapshot = {
  id: string;
  private?: boolean;
  activeTabId: string;
  tabs: TabSnapshot[];
};

export type Settings = {
  homepage: string;
  searchEngine: string;
  customSearchUrl: string;
  theme: string;
  appearance: "system" | "light" | "dark";
  sidebarVisible: "show" | "hide";
  sidebarWidth: string;
  newTabPage: "blank" | "home";
  homeUrl: string;
  language: string;
  defaultZoomLevel: string;
};

export type BrowserState = {
  bridgeVersion: string;
  windowId: string;
  isPrivate: boolean;
  activeTabId: string;
  tabs: Tab[];
  windows: WindowSnapshot[];
  history: HistoryRecord[];
  bookmarks: BookmarkRecord[];
  downloads: DownloadRecord[];
  permissions: PermissionRecord[];
  logs: LogRecord[];
  commands: BrowserCommand[];
  recentEvents: EventRecord[];
  settings: Settings;
  profilePath: string;
};

export type CommandId =
  | "tabs.create"
  | "tabs.close"
  | "tabs.reopenClosed"
  | "tabs.duplicate"
  | "tabs.pin"
  | "tabs.unpin"
  | "tabs.closeOther"
  | "tabs.closeToRight"
  | "tabs.moveToNewWindow"
  | "tabs.reload"
  | "tabs.stop"
  | "tabs.goBack"
  | "tabs.goForward"
  | "tabs.home"
  | "tabs.activateNext"
  | "tabs.activatePrevious"
  | "windows.create"
  | "windows.createPrivate"
  | "windows.close"
  | "windows.reopenClosed"
  | "app.focusOmnibox"
  | "app.openSettings"
  | "app.openHistory"
  | "app.openDownloads"
  | "app.openBookmarks"
  | "app.openDebug"
  | "app.toggleSidebar"
  | "app.openDevTools"
  | "page.find"
  | "page.stopFinding"
  | "page.zoomIn"
  | "page.zoomOut"
  | "page.zoomReset"
  | "page.print"
  | "page.viewSource"
  | "bookmarks.addActive"
  | "bookmarks.save"
  | "bookmarks.remove";

export type BridgeMethodMap = {
  "app.getState": { params: Record<string, never>; result: BrowserState };
  "commands.list": { params: Record<string, never>; result: BrowserCommand[] };
  "commands.execute": { params: { id: CommandId | string; args?: Record<string, unknown> }; result: unknown };
  "tabs.create": { params: { url?: string; active?: boolean }; result: boolean };
  "tabs.navigate": { params: { tabId: string; input: string }; result: boolean };
  "tabs.activate": { params: { tabId: string }; result: boolean };
  "tabs.close": { params: { tabId: string }; result: boolean };
  "tabs.reload": { params: { tabId: string }; result: boolean };
  "tabs.stop": { params: { tabId: string }; result: boolean };
  "tabs.goBack": { params: { tabId: string }; result: boolean };
  "tabs.goForward": { params: { tabId: string }; result: boolean };
  "tabs.move": { params: { tabId: string; toIndex: number }; result: boolean };
  "bookmarks.save": { params: { title: string; url: string; faviconUrl: string }; result: boolean };
  "bookmarks.remove": { params: { url: string }; result: boolean };
  "settings.set": { params: { key: string; value: string }; result: boolean };
};

export type EventMap = {
  "tabs.created": void;
  "tabs.updated": void;
  "tabs.closed": void;
  "tabs.activated": void;
  "navigation.started": { tabId: string; url: string };
  "navigation.finished": { tabId: string; url: string };
  "navigation.failed": { tabId: string; url: string; errorText: string };
  "downloads.updated": void;
  "download.changed": DownloadRecord;
  "bookmark.changed": void;
  "history.changed": void;
  "setting.changed": { key: string; value: string };
  "permission.changed": void;
  "window.created": void;
  "window.closed": void;
  "window.focused": void;
  "app.stateChanged": void;
};

export const fubukiLogoSvg = `<svg width="512" height="512" viewBox="0 0 512 512" fill="none" xmlns="http://www.w3.org/2000/svg">
<path d="M128 440L183.252 248.366M470 72L252.28 72C238.617 72 226.68 81.2317 223.244 94.4554L183.252 248.366M183.252 248.366H363.904" stroke="url(#paint0_linear_7_2)" stroke-width="25" stroke-linecap="round"/>
<path d="M95.6021 142.602L148.204 195.204M148.204 195.204L43.0001 195.204M148.204 195.204L95.6021 247.806M148.204 195.204V300.408M148.204 195.204L200.806 247.806M148.204 195.204V90M148.204 195.204L200.806 142.602M148.204 195.204H253.408" stroke="#1AADEB" stroke-width="5" stroke-linecap="round"/>
<defs>
<linearGradient id="paint0_linear_7_2" x1="257.282" y1="72" x2="257.282" y2="476.326" gradientUnits="userSpaceOnUse">
<stop stop-color="#FF9686"/>
<stop offset="1" stop-color="#A7ABE0"/>
</linearGradient>
</defs>
</svg>`;

export const fubukiLogoDataUri = `data:image/svg+xml,${encodeURIComponent(fubukiLogoSvg)}`;

type NativeQuery = {
  request: string;
  onSuccess: (response: string) => void;
  onFailure: (code: number, message: string) => void;
};

declare global {
  interface Window {
    cefQuery?: (query: NativeQuery) => void;
    fubuki: {
      bridgeVersion: string;
      invoke: <T = unknown>(method: string, params?: Record<string, unknown>) => Promise<T>;
      on: (eventName: string, listener: (payload: unknown) => void) => () => void;
    };
  }
}

const listeners = new Map<string, Set<(payload: unknown) => void>>();

function emit(eventName: string, payload: unknown) {
  listeners.get(eventName)?.forEach((listener) => listener(payload));
}

window.addEventListener("fubuki:event", (event) => {
  const detail = (event as CustomEvent).detail as { name?: string; payload?: unknown };
  if (detail?.name) {
    emit(detail.name, detail.payload);
  }
});

async function invoke<T = unknown>(method: string, params: Record<string, unknown> = {}): Promise<T> {
  if (!window.cefQuery) {
    throw new Error("Fubuki native bridge is not available");
  }

  return new Promise<T>((resolve, reject) => {
    window.cefQuery?.({
      request: JSON.stringify({ version: "1", method, params }),
      onSuccess: (response) => resolve(JSON.parse(response) as T),
      onFailure: (code, message) => reject(new Error(`${code}: ${message}`))
    });
  });
}

function on(eventName: string, listener: (payload: unknown) => void): () => void {
  const set = listeners.get(eventName) ?? new Set<(payload: unknown) => void>();
  set.add(listener);
  listeners.set(eventName, set);
  return () => set.delete(listener);
}

window.fubuki = {
  bridgeVersion: "1",
  invoke,
  on
};

export const fubuki = window.fubuki;

export function invokeBridge<K extends keyof BridgeMethodMap>(
  method: K,
  params?: BridgeMethodMap[K]["params"]
): Promise<BridgeMethodMap[K]["result"]> {
  return fubuki.invoke<BridgeMethodMap[K]["result"]>(
    method,
    (params ?? {}) as Record<string, unknown>
  );
}

export function onBridgeEvent<K extends keyof EventMap>(
  eventName: K,
  listener: (payload: EventMap[K]) => void
): () => void {
  return fubuki.on(eventName, listener as (payload: unknown) => void);
}

export const commands = {
  execute: <T = unknown>(id: CommandId | string, args: Record<string, unknown> = {}) =>
    invokeBridge("commands.execute", { id, args }) as Promise<T>,
  list: () => invokeBridge("commands.list")
};

export const tabs = {
  create: (url = "fubuki://newtab/") => commands.execute<boolean>("tabs.create", { url }),
  navigate: (tabId: string, input: string) => invokeBridge("tabs.navigate", { tabId, input }),
  activate: (tabId: string) => invokeBridge("tabs.activate", { tabId }),
  close: (tabId: string) => commands.execute<boolean>("tabs.close", { tabId }),
  pin: (tabId: string, pinned: boolean) => commands.execute<boolean>(pinned ? "tabs.pin" : "tabs.unpin", { tabId }),
  duplicate: (tabId: string) => commands.execute<boolean>("tabs.duplicate", { tabId }),
  reopenClosed: () => commands.execute<boolean>("tabs.reopenClosed"),
  closeOther: (tabId: string) => commands.execute<boolean>("tabs.closeOther", { tabId }),
  closeToRight: (tabId: string) => commands.execute<boolean>("tabs.closeToRight", { tabId }),
  moveToNewWindow: (tabId: string) => commands.execute<boolean>("tabs.moveToNewWindow", { tabId })
};

export const page = {
  find: (query: string, forward = true) => commands.execute<boolean>("page.find", { query, forward }),
  stopFinding: () => commands.execute<boolean>("page.stopFinding", { clear: true }),
  zoomIn: () => commands.execute<boolean>("page.zoomIn"),
  zoomOut: () => commands.execute<boolean>("page.zoomOut"),
  zoomReset: () => commands.execute<boolean>("page.zoomReset"),
  print: () => commands.execute<boolean>("page.print"),
  viewSource: () => commands.execute<boolean>("page.viewSource")
};
