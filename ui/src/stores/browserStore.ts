import { createStore } from 'solid-js/store';
import {
  fromFrostTab,
  getBrowserState,
  invokeBridge,
  onBridgeEvent,
  type BookmarkRecord,
  type BrowserState,
  type FrostTabState,
  type HistoryRecord,
  type Tab,
} from '../bridge/fubuki';

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
  },
  profilePath: '',
  status: 'Starting',
};

export const [browserState, setBrowserState] = createStore(initialState);

// --- Lightweight targeted refresh (no full snapshot) ---

let bookmarksPending = false;
let historyPending = false;

async function refreshBookmarks() {
  if (bookmarksPending) return;
  bookmarksPending = true;
  try {
    const list = await invokeBridge('bookmarks.list');
    setBrowserState('bookmarks', list as BookmarkRecord[]);
  } catch {
    // ignore
  } finally {
    bookmarksPending = false;
  }
}

async function refreshHistory() {
  if (historyPending) return;
  historyPending = true;
  try {
    const list = await invokeBridge('history.list');
    setBrowserState('history', list as HistoryRecord[]);
  } catch {
    // ignore
  } finally {
    historyPending = false;
  }
}

// --- Full snapshot refresh (used only on startup and app.stateChanged) ---

let pendingFullRefresh: Promise<void> | undefined;
let refreshRequested = false;
let stateRevision = 0;

/**
 * Full snapshot refresh — only for startup and rare edge cases.
 * Loads one state snapshot per pass. Commands are cached separately.
 */
export async function refreshFullState(status = 'Ready') {
  refreshRequested = true;
  if (pendingFullRefresh) return pendingFullRefresh;

  pendingFullRefresh = (async () => {
    try {
      while (refreshRequested) {
        refreshRequested = false;
        const revisionBeforeRequest = stateRevision;
        const state = await getBrowserState();

        // A differential event received while the snapshot was in flight is
        // newer than that snapshot may be. Retry instead of overwriting it.
        if (revisionBeforeRequest !== stateRevision) {
          refreshRequested = true;
          continue;
        }
        applyFullState(state, status);
      }
    } catch (error) {
      console.error('[Fubuki] Full state refresh failed:', error);
      setBrowserState('status', 'Error');
    }
  })().finally(() => {
    pendingFullRefresh = undefined;
  });
  return pendingFullRefresh;
}

function applyFullState(state: BrowserState, status: string) {
  setBrowserState({ ...state, status });
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

export function activeTabId(): string {
  return browserState.activeTabId;
}

export function currentLanguage(): string {
  return browserState.settings.language;
}

// --- Actions ---

export async function toggleBookmark(): Promise<void> {
  const tab = activeTab();
  if (
    !tab?.url ||
    tab.url.startsWith('fubuki://') ||
    tab.url.startsWith('data:')
  )
    return;
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

// --- Event binding ---

export function bindNativeEvents() {
  // All tab/window events are handled by direct store patches — zero bridge calls.
  const disposers = [
    // --- Tab lifecycle (direct patches) ---
    onBridgeEvent('tab.created', (tab) => {
      stateRevision += 1;
      const nextTab = fromFrostTab(tab);
      if (nextTab.isActive) {
        setBrowserState('tabs', (item) => item.id !== nextTab.id, {
          isActive: false,
        });
      }
      setBrowserState('tabs', (tabs) => [
        ...tabs.filter((item) => item.id !== nextTab.id),
        nextTab,
      ]);
      if (nextTab.isActive) {
        setBrowserState('activeTabId', nextTab.id);
      }
    }),

    onBridgeEvent('tab.updated', (patch) => {
      stateRevision += 1;
      const tabPatch = toTabPatch(patch);
      if (tabPatch) {
        setBrowserState(
          'tabs',
          (tab) => tab.id === patch.tabId,
          (tab) => ({
            ...tab,
            ...tabPatch,
          }),
        );
      }
    }),

    onBridgeEvent('tab.closed', ({ tabId }) => {
      stateRevision += 1;
      const remaining = browserState.tabs.filter((tab) => tab.id !== tabId);
      setBrowserState('tabs', remaining);
      if (browserState.activeTabId === tabId) {
        setBrowserState('activeTabId', remaining[0]?.id ?? '');
      }
    }),

    onBridgeEvent('tab.activated', ({ tabId }) => {
      stateRevision += 1;
      setBrowserState('tabs', (tab) => tab.id === tabId, { isActive: true });
      setBrowserState('tabs', (tab) => tab.id !== tabId, { isActive: false });
      setBrowserState('activeTabId', tabId);
    }),

    // The compatibility host reports a reorder only on the plural event.
    onBridgeEvent('tabs.updated', (payload) => {
      if (payload?.message === 'reordered') {
        void refreshFullState('tabs.reordered');
      }
    }),

    // --- Window lifecycle (direct patches) ---
    onBridgeEvent('window.created', (windowState) => {
      if (windowState && 'id' in windowState) {
        stateRevision += 1;
        setBrowserState('windows', (w) => [
          ...w,
          {
            id: windowState.id,
            private: windowState.isPrivate,
            activeTabId: windowState.activeTabId ?? '',
            tabs: [],
          },
        ]);
      } else {
        // Legacy events contain windowId rather than a complete WindowState.
        void refreshFullState('window.created');
      }
    }),

    onBridgeEvent('window.closed', () => {
      // Windows changed — need full refresh to reconcile tabs
      void refreshFullState('window.closed');
    }),

    onBridgeEvent('window.focused', () => {
      // Window focus changed — need full refresh to get active window/tabs
      void refreshFullState('window.focused');
    }),

    // --- Settings (direct patch) ---
    onBridgeEvent('setting.changed', ({ key, value }) => {
      if (
        typeof key === 'string' &&
        typeof value === 'string' &&
        isSettingsKey(key)
      ) {
        stateRevision += 1;
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
      void refreshFullState('downloads.updated');
    }),

    onBridgeEvent('download.changed', () => {
      void refreshFullState('download.changed');
    }),

    // --- Full app state changed (edge cases) ---
    onBridgeEvent('app.stateChanged', () => {
      void refreshFullState('app.stateChanged');
    }),
  ];

  // Fire-and-forget initial state load
  void refreshFullState('Ready');

  return () => disposers.forEach((dispose) => dispose());
}

// --- Helpers ---

function toTabPatch(
  patch: Partial<FrostTabState> & { tabId: string },
): Partial<Tab> | null {
  // Fast path: check if any field actually changed
  const keys = Object.keys(patch) as Array<string>;
  let hasChanges = false;
  for (const key of keys) {
    if (key !== 'tabId' && key !== 'windowId') {
      hasChanges = true;
      break;
    }
  }
  if (!hasChanges) return null;

  const next: Partial<Tab> = {};
  if (patch.title !== undefined) next.title = patch.title;
  if (patch.url !== undefined) next.url = patch.url;
  if (patch.faviconUrl !== undefined) next.faviconUrl = patch.faviconUrl;
  if (patch.errorText !== undefined) next.errorText = patch.errorText;
  if (patch.zoomLevel !== undefined) next.zoomLevel = patch.zoomLevel;
  if (patch.isLoading !== undefined) next.isLoading = patch.isLoading;
  if (patch.canGoBack !== undefined) next.canGoBack = patch.canGoBack;
  if (patch.canGoForward !== undefined) next.canGoForward = patch.canGoForward;
  if (patch.isPinned !== undefined) next.isPinned = patch.isPinned;
  return Object.keys(next).length > 0 ? next : null;
}

function isSettingsKey(key: string): key is keyof BrowserState['settings'] {
  return key in browserState.settings;
}
