import { createStore } from 'solid-js/store';
import {
  invokeBridge,
  normalizeAppState,
  onBridgeEvent,
  type BookmarkRecord,
  type BrowserState,
  type DownloadRecord,
  type HistoryRecord,
  type Tab,
} from '../bridge/fubuki';
import {
  reduceBrowserEvent,
  type BrowserEvent,
  type BrowserEventResult,
} from './browserEventReducer';
export { reorderTab } from './tabUtils';

const initialState: BrowserState & { status: string } = {
  bridgeVersion: '1',
  windowId: '',
  isPrivate: false,
  activeTabId: '',
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
    startupBehavior: 'lastSession',
    downloadDirectory: '',
    askBeforeDownload: 'false',
    closeWindowWithLastTab: 'false',
  },
  profilePath: '',
  status: 'Starting',
};

export const [browserState, setBrowserState] = createStore(initialState);

// --- Lightweight targeted refresh (no full snapshot) ---

let bookmarksPending = false;
let historyPending = false;
let commandsPending = false;
let downloadsPending = false;
let bookmarksRefreshRequested = false;
let historyRefreshRequested = false;
let downloadsRefreshRequested = false;

async function refreshCommands() {
  if (commandsPending) return;
  commandsPending = true;
  try {
    setBrowserState('commands', await invokeBridge('commands.list'));
  } catch {
    // Keep the last known command list on transient bridge failures.
  } finally {
    commandsPending = false;
  }
}

async function refreshBookmarks() {
  bookmarksRefreshRequested = true;
  if (bookmarksPending) return;
  bookmarksPending = true;
  try {
    while (bookmarksRefreshRequested) {
      bookmarksRefreshRequested = false;
      const list = await invokeBridge('bookmarks.list');
      setBrowserState('bookmarks', list as BookmarkRecord[]);
    }
  } catch {
    // Keep the last known bookmark list on transient bridge failures.
  } finally {
    bookmarksPending = false;
  }
}

async function refreshHistory() {
  historyRefreshRequested = true;
  if (historyPending) return;
  historyPending = true;
  try {
    while (historyRefreshRequested) {
      historyRefreshRequested = false;
      const list = await invokeBridge('history.list');
      setBrowserState('history', list as HistoryRecord[]);
    }
  } catch {
    // Keep the last known history list on transient bridge failures.
  } finally {
    historyPending = false;
  }
}

async function refreshDownloads() {
  downloadsRefreshRequested = true;
  if (downloadsPending) return;
  downloadsPending = true;
  try {
    while (downloadsRefreshRequested) {
      downloadsRefreshRequested = false;
      const list = await invokeBridge('downloads.list');
      setBrowserState('downloads', list as DownloadRecord[]);
    }
  } catch {
    // Keep the last known download list on transient bridge failures.
  } finally {
    downloadsPending = false;
  }
}

// --- Full snapshot refresh (used only on startup and app.stateChanged) ---

let pendingFullRefresh: Promise<void> | undefined;
let requestedFullRefreshStatus: string | undefined;

/**
 * Full snapshot refresh — only for startup and rare edge cases.
 * Calls app.snapshot (1 bridge call). Commands are cached separately.
 */
export async function refreshFullState(status = 'Ready') {
  requestedFullRefreshStatus = status;
  if (pendingFullRefresh) return pendingFullRefresh;
  pendingFullRefresh = (async () => {
    while (requestedFullRefreshStatus !== undefined) {
      const nextStatus = requestedFullRefreshStatus;
      requestedFullRefreshStatus = undefined;
      try {
        const snapshot = await invokeBridge('app.snapshot');
        const state = normalizeAppState(snapshot);
        void refreshCommands();

        // Only update slices that actually changed
        if (state.activeTabId !== browserState.activeTabId) {
          setBrowserState('activeTabId', state.activeTabId);
        }
        if (state.windowId !== browserState.windowId) {
          setBrowserState('windowId', state.windowId);
        }
        if (state.isPrivate !== browserState.isPrivate) {
          setBrowserState('isPrivate', state.isPrivate);
        }
        if (state.bridgeVersion !== browserState.bridgeVersion) {
          setBrowserState('bridgeVersion', state.bridgeVersion);
        }
        if (state.tabs !== browserState.tabs) {
          setBrowserState('tabs', state.tabs);
        }
        if (state.windows !== browserState.windows) {
          setBrowserState('windows', state.windows);
        }
        if (state.settings !== browserState.settings) {
          setBrowserState('settings', state.settings);
        }
        if (state.downloads !== browserState.downloads) {
          setBrowserState('downloads', state.downloads);
        }
        if (state.history !== browserState.history) {
          setBrowserState('history', state.history);
        }
        if (state.bookmarks !== browserState.bookmarks) {
          setBrowserState('bookmarks', state.bookmarks);
        }
        if (state.permissions !== browserState.permissions) {
          setBrowserState('permissions', state.permissions);
        }
        setBrowserState('status', nextStatus);
      } catch (error) {
        console.error('[Fubuki] Full state refresh failed:', error);
        setBrowserState('status', 'Error');
      }
    }
  })().finally(() => {
    pendingFullRefresh = undefined;
    if (requestedFullRefreshStatus !== undefined) {
      void refreshFullState(requestedFullRefreshStatus);
    }
  });
  return pendingFullRefresh;
}

// Keep backward-compatible alias
export const refreshState = refreshFullState;

// --- Accessors ---

export function activeTab(): Tab | undefined {
  return browserState.tabs.find((tab) => tab.id === browserState.activeTabId);
}

export function isTabBookmarked(url: string | undefined): boolean {
  if (!url) return false;
  return browserState.bookmarks.some((bookmark) => bookmark.url === url);
}

export function isBookmarkableUrl(url: string | undefined): boolean {
  return Boolean(
    url && !url.startsWith('fubuki://') && !url.startsWith('data:'),
  );
}

export function activeTabId(): string {
  return browserState.activeTabId;
}

export function currentLanguage(): string {
  return browserState.settings.language;
}

// --- Actions ---

export async function toggleBookmark(): Promise<void> {
  const tab = activeTab();
  if (!tab?.url || !isBookmarkableUrl(tab.url)) return;
  try {
    if (isTabBookmarked(tab.url)) {
      await invokeBridge('bookmarks.remove', { url: tab.url });
    } else {
      await invokeBridge('bookmarks.save', {
        title: tab.title || tab.url,
        url: tab.url,
        faviconUrl: tab.faviconUrl || '',
      });
    }
    // Only refresh bookmarks, not the full state
    await refreshBookmarks();
  } catch (error) {
    console.error('[Fubuki] Failed to toggle bookmark:', error);
  }
}

export function toggleSidebar(): void {
  const next =
    browserState.settings.sidebarVisible === 'hide' ? 'show' : 'hide';
  // Update optimistically — no bridge refresh needed for UI state
  setBrowserState('settings', 'sidebarVisible', next);
  void invokeBridge('settings.set', {
    key: 'sidebarVisible',
    value: next,
  }).catch((error) => {
    console.error('[Fubuki] Failed to toggle sidebar:', error);
    // Revert on failure
    setBrowserState(
      'settings',
      'sidebarVisible',
      next === 'show' ? 'hide' : 'show',
    );
  });
}

export function navigateInternal(url: string): void {
  const tab = activeTab();
  const promise = tab
    ? invokeBridge('tabs.navigate', { tabId: tab.id, input: url })
    : invokeBridge('tabs.create', { url, active: true });
  void promise.catch((error) =>
    console.error('[Fubuki] Failed to navigate:', error),
  );
}

export async function clearHistory(): Promise<boolean> {
  // `history.clear` is a legacy native alias. Use the Frost Protocol method
  // directly so bulk deletion also works with the engine-only bridge.
  const cleared = await invokeBridge('history.clearRange', { range: 'all' });
  if (cleared) setBrowserState('history', []);
  return cleared;
}

export async function clearDownloadHistory(): Promise<boolean> {
  return invokeBridge('downloads.clear');
}

export async function clearBookmarks(): Promise<boolean> {
  return invokeBridge('bookmarks.clear');
}

// --- Event binding ---

export function bindNativeEvents() {
  const disposers = [
    onBridgeEvent('tab.created', (payload) =>
      applyBrowserEvent({ type: 'tab.created', payload }),
    ),
    onBridgeEvent('tab.updated', (payload) =>
      applyBrowserEvent({ type: 'tab.updated', payload }),
    ),
    onBridgeEvent('tab.closed', (payload) =>
      applyBrowserEvent({ type: 'tab.closed', payload }),
    ),
    onBridgeEvent('tab.activated', (payload) =>
      applyBrowserEvent({ type: 'tab.activated', payload }),
    ),
    onBridgeEvent('tab.moved', (payload) =>
      applyBrowserEvent({ type: 'tab.moved', payload }),
    ),
    onBridgeEvent('window.created', (payload) =>
      applyBrowserEvent({ type: 'window.created', payload }),
    ),
    onBridgeEvent('window.closed', (payload) =>
      applyBrowserEvent({ type: 'window.closed', payload }),
    ),
    onBridgeEvent('window.focused', (payload) =>
      applyBrowserEvent({ type: 'window.focused', payload }),
    ),

    // --- Settings (direct patch) ---
    onBridgeEvent('setting.changed', ({ key, value }) => {
      if (
        typeof key === 'string' &&
        typeof value === 'string' &&
        isSettingsKey(key)
      ) {
        setBrowserState('settings', key, value);
      }
    }),

    // --- Bookmarks / History (targeted single-endpoint refresh) ---
    onBridgeEvent('bookmark.changed', () => {
      void refreshBookmarks();
    }),

    onBridgeEvent('history.changed', () => {
      void refreshHistory();
    }),

    onBridgeEvent('permission.changed', () => {
      // Permissions are rarely needed in the sidebar — skip refresh.
      // Will be available on next full refresh (startup, settings page).
    }),

    // --- Downloads (targeted refresh) ---
    onBridgeEvent('downloads.updated', () => {
      void refreshDownloads();
    }),

    onBridgeEvent('download.changed', () => {
      void refreshDownloads();
    }),

    // --- Full app state changed (edge cases) ---
    onBridgeEvent('app.stateChanged', () => {
      void refreshFullState('app.stateChanged');
    }),
  ];

  // Fire-and-forget initial state load
  void refreshFullState('Ready');
  void refreshCommands();

  return () => disposers.forEach((dispose) => dispose());
}

// --- Event application ---

function applyBrowserEvent(event: BrowserEvent): void {
  const result = reduceBrowserEvent(browserState, event);
  applyBrowserEventResult(result);
  if (result.refreshSnapshot) {
    void refreshFullState(event.type);
  }
}

function applyBrowserEventResult(result: BrowserEventResult): void {
  if (!result.changed) return;
  if (result.state.tabs !== browserState.tabs) {
    setBrowserState('tabs', result.state.tabs);
  }
  if (result.state.windows !== browserState.windows) {
    setBrowserState('windows', result.state.windows);
  }
  if (result.state.activeTabId !== browserState.activeTabId) {
    setBrowserState('activeTabId', result.state.activeTabId);
  }
}

function isSettingsKey(key: string): key is keyof BrowserState['settings'] {
  return key in browserState.settings;
}
