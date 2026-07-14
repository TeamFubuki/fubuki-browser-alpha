import { afterEach, describe, expect, it, vi } from 'vitest';
import type { BrowserState, FrostTabState, Tab } from '../bridge/fubuki';

type Listener = (payload: unknown) => void;

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

function tab(id: string, active = false): Tab {
  return {
    id,
    title: id,
    url: `https://${id}.example`,
    faviconUrl: '',
    errorText: '',
    zoomLevel: 0,
    isLoading: false,
    canGoBack: false,
    canGoForward: false,
    isActive: active,
    isPinned: false,
  };
}

function frostTab(id: string, active = false): FrostTabState {
  return { ...tab(id, active), windowId: 'window-1' };
}

function state(ids: string[], activeId = ids[0] ?? ''): BrowserState {
  const tabs = ids.map((id) => tab(id, id === activeId));
  return {
    bridgeVersion: 'frost-0',
    windowId: 'window-1',
    isPrivate: false,
    activeTabId: activeId,
    tabs,
    windows: [
      {
        id: 'window-1',
        activeTabId: activeId,
        tabs: tabs.map((item) => ({
          title: item.title,
          url: item.url,
          faviconUrl: '',
          pinned: false,
          active: item.isActive,
        })),
      },
    ],
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
  };
}

function convertTab(value: FrostTabState): Tab {
  return {
    id: value.id,
    title: value.title,
    url: value.url,
    faviconUrl: value.faviconUrl,
    errorText: value.errorText,
    zoomLevel: value.zoomLevel,
    isLoading: value.isLoading,
    canGoBack: value.canGoBack,
    canGoForward: value.canGoForward,
    isActive: value.isActive,
    isPinned: value.isPinned,
  };
}

async function loadStore(states: Array<Promise<BrowserState> | BrowserState>) {
  const listeners = new Map<string, Set<Listener>>();
  const getBrowserState = vi.fn<() => Promise<BrowserState>>();
  for (const result of states) {
    getBrowserState.mockImplementationOnce(() => Promise.resolve(result));
  }
  getBrowserState.mockResolvedValue(states.at(-1) as BrowserState);

  vi.doMock('../bridge/fubuki', () => ({
    fromFrostTab: convertTab,
    getBrowserState,
    invokeBridge: vi.fn().mockResolvedValue([]),
    onBridgeEvent: (name: string, listener: Listener) => {
      const eventListeners = listeners.get(name) ?? new Set<Listener>();
      eventListeners.add(listener);
      listeners.set(name, eventListeners);
      return () => eventListeners.delete(listener);
    },
  }));

  const store = await import('../stores/browserStore');
  return {
    ...store,
    getBrowserState,
    emit(name: string, payload?: unknown) {
      listeners.get(name)?.forEach((listener) => listener(payload));
    },
  };
}

afterEach(() => {
  vi.doUnmock('../bridge/fubuki');
  vi.resetModules();
});

describe('browser store snapshot coordination', () => {
  it('does not let an in-flight stale snapshot erase a differential event', async () => {
    const first = deferred<BrowserState>();
    const store = await loadStore([
      first.promise,
      state(['old-tab', 'new-tab'], 'new-tab'),
    ]);
    const dispose = store.bindNativeEvents();
    expect(store.getBrowserState).toHaveBeenCalledTimes(1);

    store.emit('tab.created', frostTab('new-tab', true));
    expect(store.browserState.activeTabId).toBe('new-tab');
    first.resolve(state(['old-tab']));

    await vi.waitFor(() => {
      expect(store.getBrowserState).toHaveBeenCalledTimes(2);
      expect(store.browserState.tabs.map((item) => item.id)).toEqual([
        'old-tab',
        'new-tab',
      ]);
    });
    dispose();
  });

  it('queues a state-change refresh that arrives during another refresh', async () => {
    const first = deferred<BrowserState>();
    const store = await loadStore([
      first.promise,
      state(['latest-tab'], 'latest-tab'),
    ]);
    const dispose = store.bindNativeEvents();

    store.emit('app.stateChanged');
    first.resolve(state(['stale-tab'], 'stale-tab'));

    await vi.waitFor(() => {
      expect(store.getBrowserState).toHaveBeenCalledTimes(2);
      expect(store.browserState.activeTabId).toBe('latest-tab');
    });
    dispose();
  });
});
