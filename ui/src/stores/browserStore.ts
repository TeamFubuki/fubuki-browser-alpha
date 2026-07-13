import { createStore } from 'solid-js/store';
import {
  fromFrostTab,
  invokeBridge,
  normalizeAppState,
  onBridgeFailure,
  onBridgeEvent,
  requireBridgeSuccess,
  type BookmarkRecord,
  type BrowserState,
  type FrostTabState,
  type HistoryRecord,
  type Tab,
} from '../bridge/fubuki';

export type BrowserError = {
  message: string;
  method?: string;
  occurredAt: number;
};

const initialState: BrowserState & {
  status: string;
  error: BrowserError | null;
} = {
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
  error: null,
};

export const [browserState, setBrowserState] = createStore(initialState);
const reportedErrors = new WeakSet<Error>();

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function reportBrowserError(error: unknown, method?: string): void {
  if (error instanceof Error) {
    if (reportedErrors.has(error)) return;
    reportedErrors.add(error);
  }
  const message = errorMessage(error);
  console.error(`[Fubuki] ${method ?? 'UI operation'} failed:`, error);
  setBrowserState('error', {
    message,
    method,
    occurredAt: Date.now(),
  });
  setBrowserState('status', 'Error');
}

export function clearBrowserError(): void {
  setBrowserState('error', null);
  if (browserState.status === 'Error') setBrowserState('status', 'Ready');
}

/** Handles user gestures without losing rejected bridge operations. */
export function runBrowserAction(
  promise: Promise<unknown>,
  method?: string,
): void {
  void promise.catch((error) => reportBrowserError(error, method));
}

onBridgeFailure(({ method, error }) => reportBrowserError(error, method));

// --- Lightweight targeted refresh (no full snapshot) ---

let bookmarksPending = false;
let historyPending = false;
let downloadsPending = false;

async function refreshBookmarks() {
  if (bookmarksPending) return;
  bookmarksPending = true;
  try {
    const list = await invokeBridge('bookmarks.list');
    setBrowserState('bookmarks', list as BookmarkRecord[]);
  } catch (error) {
    reportBrowserError(error, 'bookmarks.list');
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
  } catch (error) {
    reportBrowserError(error, 'history.list');
  } finally {
    historyPending = false;
  }
}

async function refreshDownloads() {
  if (downloadsPending) return;
  downloadsPending = true;
  try {
    const list = await invokeBridge('downloads.list');
    setBrowserState('downloads', list as BrowserState['downloads']);
  } catch (error) {
    reportBrowserError(error, 'downloads.list');
  } finally {
    downloadsPending = false;
  }
}

// --- Full snapshot refresh (startup and explicit failure recovery only) ---

let pendingFullRefresh: Promise<void> | undefined;
let fullRefreshCounter = 0;

/**
 * Full snapshot refresh. Event handlers patch individual state slices and must
 * not call this as a routine synchronization mechanism.
 */
export async function refreshFullState(status = 'Ready') {
  if (pendingFullRefresh) return pendingFullRefresh;
  const myCounter = ++fullRefreshCounter;
  pendingFullRefresh = (async () => {
    try {
      const [snapshot, commandList] = await Promise.all([
        invokeBridge('app.snapshot'),
        invokeBridge('commands.list'),
      ]);
      const state = normalizeAppState(snapshot);
      state.commands = commandList;
      if (myCounter !== fullRefreshCounter) return;

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
      if (state.commands !== browserState.commands) {
        setBrowserState('commands', state.commands);
      }
      setBrowserState('status', status);
    } catch (error) {
      reportBrowserError(error, 'app.snapshot');
      throw error;
    }
  })().finally(() => {
    pendingFullRefresh = undefined;
  });
  return pendingFullRefresh;
}

/** Reconcile from Rust only after a user-visible bridge failure. */
export async function recoverFromBridgeFailure(): Promise<void> {
  await refreshFullState('Recovered');
  clearBrowserError();
}

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
  if (isTabBookmarked(tab.url)) {
    requireBridgeSuccess(
      await invokeBridge('bookmarks.remove', { url: tab.url }),
      'bookmarks.remove',
    );
  } else {
    requireBridgeSuccess(
      await invokeBridge('bookmarks.save', {
        title: tab.title || tab.url,
        url: tab.url,
        faviconUrl: tab.faviconUrl || '',
      }),
      'bookmarks.save',
    );
  }
  // Only refresh bookmarks, not the full state.
  await refreshBookmarks();
}

export async function toggleSidebar(): Promise<void> {
  const next =
    browserState.settings.sidebarVisible === 'hide' ? 'show' : 'hide';
  requireBridgeSuccess(
    await invokeBridge('settings.set', {
      key: 'sidebarVisible',
      value: next,
    }),
    'settings.set',
  );
  // The Rust acknowledgement is authoritative. The setting.changed event
  // normally performs this patch; this covers hosts that omit the echo event.
  setBrowserState('settings', 'sidebarVisible', next);
}

export async function navigateInternal(url: string): Promise<void> {
  const tab = activeTab();
  if (tab) {
    requireBridgeSuccess(
      await invokeBridge('tabs.navigate', { tabId: tab.id, input: url }),
      'tabs.navigate',
    );
  } else {
    requireBridgeSuccess(
      await invokeBridge('tabs.create', { url, active: true }),
      'tabs.create',
    );
  }
}

// --- Event binding ---

export function bindNativeEvents() {
  // All tab/window events are handled by direct store patches — zero bridge calls.
  const disposers = [
    // --- Tab lifecycle (direct patches) ---
    onBridgeEvent('tab.created', (tab) => {
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
      const remaining = browserState.tabs.filter((tab) => tab.id !== tabId);
      setBrowserState('tabs', remaining);
      if (browserState.activeTabId === tabId) {
        setBrowserState('activeTabId', remaining[0]?.id ?? '');
      }
    }),

    onBridgeEvent('tab.activated', ({ tabId }) => {
      setBrowserState('tabs', (tab) => tab.id === tabId, { isActive: true });
      setBrowserState('tabs', (tab) => tab.id !== tabId, { isActive: false });
      setBrowserState('activeTabId', tabId);
    }),

    // --- Window lifecycle (direct patches) ---
    onBridgeEvent('window.created', (windowState) => {
      if (windowState) {
        setBrowserState('windows', (w) => [
          ...w,
          {
            id: windowState.id,
            private: windowState.isPrivate,
            activeTabId: windowState.activeTabId ?? '',
            tabs: [],
          },
        ]);
      }
    }),

    onBridgeEvent('window.closed', ({ windowId }) => {
      setBrowserState('windows', (windows) =>
        windows.filter((windowState) => windowState.id !== windowId),
      );
    }),

    onBridgeEvent('window.focused', ({ windowId }) => {
      setBrowserState('windowId', windowId);
    }),

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
      runBrowserAction(refreshBookmarks(), 'bookmarks.list');
    }),

    onBridgeEvent('history.changed', () => {
      runBrowserAction(refreshHistory(), 'history.list');
    }),

    onBridgeEvent('permission.changed', () => {
      // Permissions are rarely needed in the sidebar — skip refresh.
      // Will be available on next full refresh (startup, settings page).
    }),

    // --- Downloads (targeted refresh) ---
    onBridgeEvent('downloads.updated', () => {
      runBrowserAction(refreshDownloads(), 'downloads.list');
    }),

    onBridgeEvent('download.changed', () => {
      runBrowserAction(refreshDownloads(), 'downloads.list');
    }),

    // app.stateChanged is a legacy compatibility notification. Correct Frost
    // hosts emit individual differential events; snapshot is reserved for an
    // explicit failure-recovery action.
    onBridgeEvent('app.stateChanged', () => {
      setBrowserState('status', 'Ready');
    }),
  ];

  // Startup is the sole automatic full snapshot.
  runBrowserAction(refreshFullState('Ready'), 'app.snapshot');

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
