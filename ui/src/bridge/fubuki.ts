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

export type FrostTabState = {
  id: string;
  windowId: string;
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

export type FrostWindowState = {
  id: string;
  activeTabId: string | null;
  isPrivate: boolean;
  tabIds: string[];
};

export type FrostAppState = {
  protocolVersion: number;
  activeWindowId: string | null;
  windows: FrostWindowState[];
  tabs: FrostTabState[];
  settings?: Partial<Settings>;
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
  appearance: 'system' | 'light' | 'dark';
  sidebarVisible: 'show' | 'hide';
  sidebarWidth: string;
  newTabPage: 'blank' | 'home';
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
  | 'tabs.create'
  | 'tabs.close'
  | 'tabs.reopenClosed'
  | 'tabs.duplicate'
  | 'tabs.pin'
  | 'tabs.unpin'
  | 'tabs.closeOther'
  | 'tabs.closeToRight'
  | 'tabs.moveToNewWindow'
  | 'tabs.reload'
  | 'tabs.stop'
  | 'tabs.goBack'
  | 'tabs.goForward'
  | 'tabs.home'
  | 'tabs.activateNext'
  | 'tabs.activatePrevious'
  | 'windows.create'
  | 'windows.createPrivate'
  | 'windows.close'
  | 'windows.reopenClosed'
  | 'app.focusOmnibox'
  | 'app.openSettings'
  | 'app.openHistory'
  | 'app.openDownloads'
  | 'app.openBookmarks'
  | 'app.openDebug'
  | 'app.toggleSidebar'
  | 'app.openDevTools'
  | 'page.find'
  | 'page.stopFinding'
  | 'page.zoomIn'
  | 'page.zoomOut'
  | 'page.zoomReset'
  | 'page.print'
  | 'page.viewSource'
  | 'bookmarks.addActive'
  | 'bookmarks.save'
  | 'bookmarks.remove';

export type BridgeMethodMap = {
  'app.getState': { params: Record<string, never>; result: BrowserState };
  'app.snapshot': {
    params: Record<string, never>;
    result: FrostAppState | BrowserState;
  };
  'commands.list': { params: Record<string, never>; result: BrowserCommand[] };
  'commands.execute': {
    params: { id: CommandId | string; args?: Record<string, unknown> };
    result: unknown;
  };
  'tabs.create': {
    params: { url?: string; active?: boolean };
    result: boolean;
  };
  'tabs.list': { params: Record<string, never>; result: FrostTabState[] | Tab[] };
  'tabs.navigate': {
    params: { tabId: string; input: string };
    result: boolean;
  };
  'tabs.activate': { params: { tabId: string }; result: boolean };
  'tabs.close': { params: { tabId: string }; result: boolean };
  'tabs.reload': { params: { tabId: string }; result: boolean };
  'tabs.stop': { params: { tabId: string }; result: boolean };
  'tabs.goBack': { params: { tabId: string }; result: boolean };
  'tabs.goForward': { params: { tabId: string }; result: boolean };
  'tabs.move': { params: { tabId: string; toIndex: number }; result: boolean };
  'windows.list': {
    params: Record<string, never>;
    result: FrostWindowState[] | WindowSnapshot[];
  };
  'windows.create': { params: Record<string, never>; result: boolean };
  'windows.close': { params: { windowId?: string }; result: boolean };
  'bookmarks.save': {
    params: { title: string; url: string; faviconUrl: string };
    result: boolean;
  };
  'bookmarks.list': { params: Record<string, never>; result: BookmarkRecord[] };
  'bookmarks.remove': { params: { url: string }; result: boolean };
  'history.list': { params: Record<string, never>; result: HistoryRecord[] };
  'history.remove': { params: { url: string }; result: boolean };
  'history.clearRange': {
    params: { range: 'lastHour' | 'today' | 'all' };
    result: boolean;
  };
  'downloads.list': { params: Record<string, never>; result: DownloadRecord[] };
  'downloads.remove': {
    params: { url?: string; path?: string };
    result: boolean;
  };
  'downloads.open': { params: { path: string }; result: boolean };
  'downloads.reveal': { params: { path: string }; result: boolean };
  'settings.get': { params: { key: string }; result: string | null };
  'settings.set': { params: { key: string; value: string }; result: boolean };
};

export type EventMap = {
  'tab.created': FrostTabState;
  'tab.updated': Partial<FrostTabState> & { tabId: string };
  'tab.closed': { tabId: string };
  'tab.activated': { tabId: string };
  'tabs.created': void;
  'tabs.updated': void;
  'tabs.closed': void;
  'tabs.activated': void;
  'navigation.started': { tabId: string; url: string };
  'navigation.finished': { tabId: string; url: string };
  'navigation.failed': { tabId: string; url: string; errorText: string };
  'downloads.updated': void;
  'download.changed': Partial<DownloadRecord> | void;
  'bookmark.changed': { url?: string } | void;
  'history.changed': { url?: string } | void;
  'setting.changed': { key: string; value: string };
  'permission.changed': void;
  'window.created': FrostWindowState | void;
  'window.closed': void;
  'window.focused': void;
  'app.stateChanged': void;
};

export { fubukiLogoSvg, fubukiLogoDataUri } from '../assets/logo';

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
      invoke: <T = unknown>(
        method: string,
        params?: Record<string, unknown>,
      ) => Promise<T>;
      on: (
        eventName: string,
        listener: (payload: unknown) => void,
      ) => () => void;
    };
  }
}

const listeners = new Map<string, Set<(payload: unknown) => void>>();

function emit(eventName: string, payload: unknown) {
  listeners.get(eventName)?.forEach((listener) => listener(payload));
}

window.addEventListener('fubuki:event', (event) => {
  const detail = (event as CustomEvent).detail as {
    name?: string;
    payload?: unknown;
  };
  if (detail?.name) {
    emit(detail.name, detail.payload);
  }
});

async function invoke<T = unknown>(
  method: string,
  params: Record<string, unknown> = {},
): Promise<T> {
  if (!window.cefQuery) {
    throw new Error('Fubuki native bridge is not available');
  }

  return new Promise<T>((resolve, reject) => {
    window.cefQuery?.({
      request: JSON.stringify({ version: 0, bridgeVersion: '1', method, params }),
      onSuccess: (response) => resolve(JSON.parse(response) as T),
      onFailure: (code, message) => reject(new Error(`${code}: ${message}`)),
    });
  });
}

function on(
  eventName: string,
  listener: (payload: unknown) => void,
): () => void {
  const set = listeners.get(eventName) ?? new Set<(payload: unknown) => void>();
  set.add(listener);
  listeners.set(eventName, set);
  return () => set.delete(listener);
}

window.fubuki = {
  bridgeVersion: '1',
  invoke,
  on,
};

export const fubuki = window.fubuki;

export function invokeBridge<K extends keyof BridgeMethodMap>(
  method: K,
  params?: BridgeMethodMap[K]['params'],
): Promise<BridgeMethodMap[K]['result']> {
  return fubuki.invoke<BridgeMethodMap[K]['result']>(
    method,
    (params ?? {}) as Record<string, unknown>,
  );
}

function isFrostAppState(
  value: FrostAppState | BrowserState,
): value is FrostAppState {
  return 'protocolVersion' in value && 'activeWindowId' in value;
}

export function fromFrostTab(tab: FrostTabState): Tab {
  return {
    id: tab.id,
    title: tab.title,
    url: tab.url,
    faviconUrl: tab.faviconUrl,
    errorText: tab.errorText,
    zoomLevel: tab.zoomLevel,
    isLoading: tab.isLoading,
    canGoBack: tab.canGoBack,
    canGoForward: tab.canGoForward,
    isActive: tab.isActive,
    isPinned: tab.isPinned,
  };
}

function fromFrostWindow(
  windowState: FrostWindowState,
  tabs: FrostTabState[],
): WindowSnapshot {
  return {
    id: windowState.id,
    private: windowState.isPrivate,
    activeTabId: windowState.activeTabId ?? '',
    tabs: windowState.tabIds.map((tabId) => {
      const tab = tabs.find((item) => item.id === tabId);
      return {
        title: tab?.title ?? 'New Tab',
        url: tab?.url ?? 'fubuki://newtab/',
        faviconUrl: tab?.faviconUrl ?? '',
        pinned: tab?.isPinned ?? false,
        active: windowState.activeTabId === tabId,
      };
    }),
  };
}

export function normalizeAppState(
  snapshot: FrostAppState | BrowserState,
): BrowserState {
  if (!isFrostAppState(snapshot)) {
    return snapshot;
  }

  const activeWindow = snapshot.windows.find(
    (windowState) => windowState.id === snapshot.activeWindowId,
  );
  const activeTab = snapshot.tabs.find((tab) => tab.isActive);

  return {
    bridgeVersion: `frost-${snapshot.protocolVersion}`,
    windowId: activeWindow?.id ?? '',
    isPrivate: activeWindow?.isPrivate ?? false,
    activeTabId: activeWindow?.activeTabId ?? activeTab?.id ?? '',
    tabs: snapshot.tabs.map(fromFrostTab),
    windows: snapshot.windows.map((windowState) =>
      fromFrostWindow(windowState, snapshot.tabs),
    ),
    history: [],
    bookmarks: [],
    downloads: [],
    permissions: [],
    logs: [],
    commands: [],
    recentEvents: [],
    settings: {
      homepage: 'https://example.com',
      searchEngine: 'google',
      customSearchUrl: 'https://www.google.com/search?q={query}',
      theme: 'light',
      appearance: 'system',
      sidebarVisible: 'show',
      sidebarWidth: '196',
      newTabPage: 'blank',
      homeUrl: 'https://example.com',
      language: 'system',
      defaultZoomLevel: '0',
      ...snapshot.settings,
    },
    profilePath: '',
  };
}

export async function getBrowserState(): Promise<BrowserState> {
  try {
    return normalizeAppState(await invokeBridge('app.snapshot'));
  } catch {
    return invokeBridge('app.getState');
  }
}

export function onBridgeEvent<K extends keyof EventMap>(
  eventName: K,
  listener: (payload: EventMap[K]) => void,
): () => void {
  return fubuki.on(eventName, listener as (payload: unknown) => void);
}

export const commands = {
  execute: <T = unknown>(
    id: CommandId | string,
    args: Record<string, unknown> = {},
  ) => invokeBridge('commands.execute', { id, args }) as Promise<T>,
  list: () => invokeBridge('commands.list'),
};

export const tabs = {
  create: (url = 'fubuki://newtab/') =>
    commands.execute<boolean>('tabs.create', { url }),
  navigate: (tabId: string, input: string) =>
    invokeBridge('tabs.navigate', { tabId, input }),
  activate: (tabId: string) => invokeBridge('tabs.activate', { tabId }),
  close: (tabId: string) => commands.execute<boolean>('tabs.close', { tabId }),
  pin: (tabId: string, pinned: boolean) =>
    commands.execute<boolean>(pinned ? 'tabs.pin' : 'tabs.unpin', { tabId }),
  duplicate: (tabId: string) =>
    commands.execute<boolean>('tabs.duplicate', { tabId }),
  reopenClosed: () => commands.execute<boolean>('tabs.reopenClosed'),
  closeOther: (tabId: string) =>
    commands.execute<boolean>('tabs.closeOther', { tabId }),
  closeToRight: (tabId: string) =>
    commands.execute<boolean>('tabs.closeToRight', { tabId }),
  moveToNewWindow: (tabId: string) =>
    commands.execute<boolean>('tabs.moveToNewWindow', { tabId }),
};

export const page = {
  find: (query: string, forward = true) =>
    commands.execute<boolean>('page.find', { query, forward }),
  stopFinding: () =>
    commands.execute<boolean>('page.stopFinding', { clear: true }),
  zoomIn: () => commands.execute<boolean>('page.zoomIn'),
  zoomOut: () => commands.execute<boolean>('page.zoomOut'),
  zoomReset: () => commands.execute<boolean>('page.zoomReset'),
  print: () => commands.execute<boolean>('page.print'),
  viewSource: () => commands.execute<boolean>('page.viewSource'),
};
