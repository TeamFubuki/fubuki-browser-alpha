import { createStore } from 'solid-js/store';
import {
  getBrowserState,
  invokeBridge,
  onBridgeEvent,
  type BrowserState,
  type EventMap,
  type FrostTabState,
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

let pendingRefresh: Promise<void> | undefined;
let refreshCounter = 0;
let lastStatus = 'Ready';

export async function refreshState(status = 'Ready') {
  lastStatus = status;
  if (pendingRefresh) {
    return pendingRefresh;
  }
  const myCounter = ++refreshCounter;
  const statusAtStart = lastStatus;
  pendingRefresh = getBrowserState()
    .then((state) => {
      // Only apply if no newer refresh has started
      if (myCounter === refreshCounter) {
        setBrowserState({ ...state, status: statusAtStart });
      }
    })
    .catch((error) => {
      console.error('[Fubuki] Failed to refresh state:', error);
      setBrowserState({ status: 'Error' });
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
    await refreshState('bookmarks.changed');
  } catch (error) {
    console.error('[Fubuki] Failed to toggle bookmark:', error);
  }
}

export function toggleSidebar(): void {
  const next =
    browserState.settings.sidebarVisible === 'hide' ? 'show' : 'hide';
  void invokeBridge('settings.set', { key: 'sidebarVisible', value: next })
    .then(() => refreshState('settings.saved'))
    .catch((error) =>
      console.error('[Fubuki] Failed to toggle sidebar:', error),
    );
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

export function bindNativeEvents() {
  const frostDisposers = [
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
        setBrowserState({ activeTabId: nextTab.id });
      }
      setBrowserState({ status: 'tab.created' });
    }),
    onBridgeEvent('tab.updated', (patch) => {
      setBrowserState('tabs', (tab) => tab.id === patch.tabId, (tab) => ({
        ...tab,
        ...toTabPatch(patch),
      }));
      setBrowserState({ status: 'tab.updated' });
    }),
    onBridgeEvent('tab.closed', ({ tabId }) => {
      const remainingTabs = browserState.tabs.filter((tab) => tab.id !== tabId);
      setBrowserState('tabs', remainingTabs);
      if (browserState.activeTabId === tabId) {
        setBrowserState({ activeTabId: remainingTabs[0]?.id ?? '' });
      }
      setBrowserState({ status: 'tab.closed' });
    }),
    onBridgeEvent('tab.activated', ({ tabId }) => {
      setBrowserState('tabs', (tab) => tab.id === tabId, { isActive: true });
      setBrowserState('tabs', (tab) => tab.id !== tabId, { isActive: false });
      setBrowserState({ activeTabId: tabId, status: 'tab.activated' });
    }),
    onBridgeEvent('window.created', (windowState) => {
      if (windowState) {
        setBrowserState('windows', (windows) => [
          ...windows,
          {
            id: windowState.id,
            private: windowState.isPrivate,
            activeTabId: windowState.activeTabId ?? '',
            tabs: [],
          },
        ]);
      }
      setBrowserState({ status: 'window.created' });
    }),
    onBridgeEvent('setting.changed', ({ key, value }) => {
      if (typeof key === 'string' && typeof value === 'string' && isSettingsKey(key)) {
        setBrowserState('settings', key, value);
      }
      setBrowserState({ status: 'setting.changed' });
    }),
  ];

  const refreshEvents: Array<keyof EventMap> = [
    'tabs.created',
    'tabs.updated',
    'tabs.closed',
    'tabs.activated',
    'navigation.started',
    'navigation.finished',
    'navigation.failed',
    'downloads.updated',
    'download.changed',
    'bookmark.changed',
    'history.changed',
    'permission.changed',
    'window.closed',
    'window.focused',
    'app.stateChanged',
  ];

  const disposers = refreshEvents.map((eventName) =>
    onBridgeEvent(eventName, () => {
      void refreshState(eventName);
    }),
  );

  void refreshState('Ready');
  return () => [...frostDisposers, ...disposers].forEach((dispose) => dispose());
}

function fromFrostTab(tab: FrostTabState): Tab {
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

function toTabPatch(
  patch: Partial<FrostTabState> & { tabId: string },
): Partial<Tab> {
  const next: Partial<Tab> = {};
  if (patch.title !== undefined) next.title = patch.title;
  if (patch.url !== undefined) next.url = patch.url;
  if (patch.faviconUrl !== undefined) next.faviconUrl = patch.faviconUrl;
  if (patch.errorText !== undefined) next.errorText = patch.errorText;
  if (patch.zoomLevel !== undefined) next.zoomLevel = patch.zoomLevel;
  if (patch.isLoading !== undefined) next.isLoading = patch.isLoading;
  if (patch.canGoBack !== undefined) next.canGoBack = patch.canGoBack;
  if (patch.canGoForward !== undefined) {
    next.canGoForward = patch.canGoForward;
  }
  if (patch.isPinned !== undefined) next.isPinned = patch.isPinned;
  return next;
}

function isSettingsKey(key: string): key is keyof BrowserState['settings'] {
  return key in browserState.settings;
}
