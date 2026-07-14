import { afterEach, describe, expect, it, vi } from 'vitest';
import type { BrowserState, FrostTabState, Tab } from '../bridge/fubuki';

type Listener = (payload: unknown) => void;
type Invoke = (method: string) => Promise<unknown>;

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

function state(ids: string[], activeId = ids[0] ?? ''): BrowserState {
  const tabs = ids.map((id) => tab(id, id === activeId));
  return {
    bridgeVersion: 'frost-0',
    windowId: 'window-1',
    isPrivate: false,
    activeTabId: activeId,
    tabs,
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
  };
}

async function loadStore(invoke: Invoke) {
  const listeners = new Map<string, Listener>();
  const invokeBridge = vi.fn(invoke);

  vi.doMock('../bridge/fubuki', () => ({
    fromFrostTab: (value: FrostTabState) => {
      const { windowId, ...result } = value;
      void windowId;
      return result;
    },
    invokeBridge,
    normalizeAppState: (value: BrowserState) => value,
    onBridgeFailure: () => () => undefined,
    onBridgeEvent: (name: string, listener: Listener) => {
      listeners.set(name, listener);
      return () => listeners.delete(name);
    },
    requireBridgeSuccess: (value: unknown) => value,
  }));

  const store = await import('../stores/browserStore');
  return {
    ...store,
    invokeBridge,
    emit(name: string, payload?: unknown) {
      listeners.get(name)?.(payload);
    },
  };
}

afterEach(() => {
  vi.doUnmock('../bridge/fubuki');
  vi.resetModules();
  vi.restoreAllMocks();
});

describe.sequential('browser store refresh coordination', () => {
  it('retries instead of applying a snapshot older than a tab event', async () => {
    const firstSnapshot = deferred<BrowserState>();
    const freshState = state(['old-tab', 'new-tab'], 'new-tab');
    let snapshotCalls = 0;
    const harness = await loadStore(async (method) => {
      if (method === 'commands.list') return [];
      if (method === 'app.snapshot') {
        snapshotCalls += 1;
        return snapshotCalls === 1 ? firstSnapshot.promise : freshState;
      }
      return [];
    });
    const dispose = harness.bindNativeEvents();

    harness.emit('tab.created', {
      ...tab('new-tab', true),
      windowId: 'window-1',
    } satisfies FrostTabState);
    expect(harness.browserState.activeTabId).toBe('new-tab');
    firstSnapshot.resolve(state(['old-tab'], 'old-tab'));

    await vi.waitFor(() => {
      expect(snapshotCalls).toBe(2);
      expect(harness.browserState.tabs.map((item) => item.id)).toEqual([
        'old-tab',
        'new-tab',
      ]);
    });
    dispose();
  });

  const targets = [
    {
      label: 'bookmarks',
      event: 'bookmark.changed',
      method: 'bookmarks.list',
      key: 'bookmarks',
      record: {
        title: 'new',
        url: 'https://new.example',
        faviconUrl: '',
        createdAt: '',
      },
    },
    {
      label: 'history',
      event: 'history.changed',
      method: 'history.list',
      key: 'history',
      record: {
        title: 'new',
        url: 'https://new.example',
        faviconUrl: '',
        createdAt: '',
      },
    },
    {
      label: 'downloads',
      event: 'download.changed',
      method: 'downloads.list',
      key: 'downloads',
      record: {
        url: 'https://new.example',
        path: '/tmp/new',
        state: 'complete',
        percent: 100,
        createdAt: '',
      },
    },
  ] as const;

  it.each(targets)(
    'queues a second $label refresh while one is pending',
    async (target) => {
      const firstList = deferred<unknown[]>();
      let listCalls = 0;
      const harness = await loadStore(async (method) => {
        if (method === 'app.snapshot') return state([]);
        if (method === 'commands.list') return [];
        if (method === target.method) {
          listCalls += 1;
          return listCalls === 1 ? firstList.promise : [target.record];
        }
        return [];
      });
      const dispose = harness.bindNativeEvents();
      await vi.waitFor(() => expect(harness.browserState.status).toBe('Ready'));

      harness.emit(target.event);
      harness.emit(target.event);
      firstList.resolve([]);

      await vi.waitFor(() => {
        expect(listCalls).toBe(2);
        expect(harness.browserState[target.key][0]?.url).toBe(
          target.record.url,
        );
      });
      dispose();
    },
  );
});
