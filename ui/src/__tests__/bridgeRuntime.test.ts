import { afterEach, describe, expect, it, vi } from 'vitest';
import type { FrostAppState } from '../bridge/fubuki';

type NativeQueryRequest = {
  request: string;
  onSuccess: (response: string) => void;
  onFailure: (code: number, message: string) => void;
};

type NativeQuery = (request: NativeQueryRequest) => void;

function installWindow(cefQuery: NativeQuery) {
  vi.stubGlobal('window', { addEventListener: vi.fn(), cefQuery });
}

afterEach(() => {
  vi.resetModules();
  vi.unstubAllGlobals();
});

describe('native bridge transport', () => {
  it('rejects malformed success responses instead of staying pending', async () => {
    installWindow(({ onSuccess }) => onSuccess('{invalid'));
    const { fubuki } = await import('../bridge/fubuki');

    await expect(fubuki.invoke('tabs.close')).rejects.toThrow(
      'returned invalid JSON',
    );
  });

  it('rejects native error envelopes delivered through onSuccess', async () => {
    installWindow(({ onSuccess }) =>
      onSuccess(JSON.stringify({ ok: false, error: 'unknown method' })),
    );
    const { fubuki } = await import('../bridge/fubuki');

    await expect(fubuki.invoke('missing.method')).rejects.toThrow(
      'unknown method',
    );
  });

  it('captures cefQuery once so frame teardown cannot orphan the promise', async () => {
    const query: NativeQuery = ({ onSuccess }) => onSuccess('true');
    let reads = 0;
    vi.stubGlobal('window', {
      addEventListener: vi.fn(),
      get cefQuery() {
        reads += 1;
        return reads === 1 ? query : undefined;
      },
    });
    const { fubuki } = await import('../bridge/fubuki');

    await expect(fubuki.invoke('tabs.close')).resolves.toBe(true);
    expect(reads).toBe(1);
  });
});

describe('browser state initialization', () => {
  it('loads and caches commands alongside Frost snapshots', async () => {
    const snapshot: FrostAppState = {
      protocolVersion: 0,
      activeWindowId: 'window-1',
      windows: [
        {
          id: 'window-1',
          activeTabId: 'tab-1',
          isPrivate: false,
          tabIds: ['tab-1'],
        },
      ],
      tabs: [
        {
          id: 'tab-1',
          windowId: 'window-1',
          title: 'New Tab',
          url: 'fubuki://newtab/',
          faviconUrl: '',
          errorText: '',
          zoomLevel: 0,
          isLoading: false,
          canGoBack: false,
          canGoForward: false,
          isActive: true,
          isPinned: false,
        },
      ],
    };
    let commandRequests = 0;
    installWindow(({ request, onSuccess }) => {
      const { method } = JSON.parse(request) as { method: string };
      if (method === 'commands.list') {
        commandRequests += 1;
        onSuccess(
          JSON.stringify([
            {
              id: 'tabs.create',
              title: 'New Tab',
              category: 'Tabs',
              shortcut: 'Cmd+T',
            },
          ]),
        );
        return;
      }
      onSuccess(JSON.stringify(snapshot));
    });
    const { getBrowserState } = await import('../bridge/fubuki');

    expect((await getBrowserState()).commands[0]?.id).toBe('tabs.create');
    expect((await getBrowserState()).commands[0]?.id).toBe('tabs.create');
    expect(commandRequests).toBe(1);
  });
});
