import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  BRIDGE_TIMEOUT_MS,
  invokeNativeBridge,
  notifyBridgeListeners,
  type NativeQuery,
} from '../bridge/runtime';
import {
  validateBridgeEvent,
  validateBridgeResponse,
} from '../bridge/validation';

const tab = {
  id: 'tab-1',
  windowId: 'window-1',
  title: 'Example',
  url: 'https://example.com',
  faviconUrl: '',
  errorText: '',
  zoomLevel: 0,
  isLoading: false,
  canGoBack: false,
  canGoForward: false,
  isActive: true,
  isPinned: false,
};

const snapshot = {
  protocolVersion: 0,
  currentWindowId: 'window-1',
  activeWindowId: 'window-1',
  windows: [
    {
      id: 'window-1',
      activeTabId: 'tab-1',
      isPrivate: false,
      tabIds: ['tab-1'],
    },
  ],
  tabs: [tab],
  history: [],
  bookmarks: [],
  downloads: [],
  permissions: [],
  settings: { appearance: 'system' },
};

afterEach(() => {
  vi.useRealTimers();
});

describe('native bridge runtime', () => {
  it('resolves a validated response', async () => {
    const result = invokeNativeBridge<boolean>((query) => {
      query.onSuccess('true');
    }, 'tabs.close');
    await expect(result).resolves.toBe(true);
  });

  it('rejects malformed JSON immediately with the method name', async () => {
    const result = invokeNativeBridge((query) => {
      query.onSuccess('{broken');
    }, 'app.snapshot');
    await expect(result).rejects.toThrow(/app\.snapshot.*JSON/i);
  });

  it('rejects after the ten second timeout', async () => {
    vi.useFakeTimers();
    const result = invokeNativeBridge(() => {}, 'history.list');
    const rejection = expect(result).rejects.toThrow(
      `history.list" timed out after ${BRIDGE_TIMEOUT_MS}ms`,
    );
    await vi.advanceTimersByTimeAsync(BRIDGE_TIMEOUT_MS);
    await rejection;
  });

  it('does not time out before ten seconds', async () => {
    vi.useFakeTimers();
    let settled = false;
    void invokeNativeBridge(() => {}, 'history.list').catch(() => {
      settled = true;
    });
    await vi.advanceTimersByTimeAsync(BRIDGE_TIMEOUT_MS - 1);
    expect(settled).toBe(false);
  });

  it('ignores callbacks arriving after timeout', async () => {
    vi.useFakeTimers();
    let pending: NativeQuery | undefined;
    const result = invokeNativeBridge((query) => {
      pending = query;
    }, 'tabs.close');
    const rejection = expect(result).rejects.toThrow(/timed out/);
    await vi.advanceTimersByTimeAsync(BRIDGE_TIMEOUT_MS);
    pending?.onSuccess('true');
    await rejection;
  });

  it('includes the method in native failure errors', async () => {
    const result = invokeNativeBridge((query) => {
      query.onFailure(500, 'unavailable');
    }, 'downloads.list');
    await expect(result).rejects.toThrow(/downloads\.list.*500.*unavailable/);
  });

  it('rejects native error envelopes', async () => {
    const result = invokeNativeBridge((query) => {
      query.onSuccess('{"ok":false,"error":"denied"}');
    }, 'tabs.close');
    await expect(result).rejects.toThrow(/tabs\.close.*denied/);
  });

  it('rejects synchronous cefQuery exceptions', async () => {
    const result = invokeNativeBridge(() => {
      throw new Error('CEF crashed');
    }, 'tabs.close');
    await expect(result).rejects.toThrow('CEF crashed');
  });
});

describe('response validation', () => {
  it('accepts a valid Frost snapshot', () => {
    expect(validateBridgeResponse('app.snapshot', snapshot)).toEqual(snapshot);
  });

  it('rejects an empty tab id in a snapshot', () => {
    expect(() =>
      validateBridgeResponse('app.snapshot', {
        ...snapshot,
        tabs: [{ ...tab, id: '' }],
      }),
    ).toThrow(/app\.snapshot.*tabs\[0\]\.id/);
  });

  it('rejects NaN zoom levels', () => {
    expect(() =>
      validateBridgeResponse('tabs.list', [{ ...tab, zoomLevel: Number.NaN }]),
    ).toThrow(/tabs\.list.*zoomLevel/);
  });

  it('rejects infinite zoom levels', () => {
    expect(() =>
      validateBridgeResponse('tabs.list', [
        { ...tab, zoomLevel: Number.POSITIVE_INFINITY },
      ]),
    ).toThrow(/finite number/);
  });

  it('rejects unknown appearance values', () => {
    expect(() =>
      validateBridgeResponse('app.snapshot', {
        ...snapshot,
        settings: { appearance: 'neon' },
      }),
    ).toThrow(/appearance/);
  });

  it('safely clamps download percentages', () => {
    const result = validateBridgeResponse('downloads.list', [
      {
        url: 'https://example.com/file.zip',
        path: '/tmp/file.zip',
        state: 'in_progress',
        percent: 140,
        createdAt: '2026-07-23',
      },
    ]) as Array<{ percent: number }>;
    expect(result[0].percent).toBe(100);
  });

  it('rejects non-finite download percentages', () => {
    expect(() =>
      validateBridgeResponse('downloads.list', [
        {
          url: 'https://example.com/file.zip',
          path: '/tmp/file.zip',
          state: 'in_progress',
          percent: Number.NaN,
          createdAt: '2026-07-23',
        },
      ]),
    ).toThrow(/downloads\.list.*percent/);
  });

  it('rejects history records without a URL', () => {
    expect(() =>
      validateBridgeResponse('history.list', [
        { title: 'Missing URL', faviconUrl: '', createdAt: 'today' },
      ]),
    ).toThrow(/history\.list.*url/);
  });

  it('rejects bookmark records with an empty URL', () => {
    expect(() =>
      validateBridgeResponse('bookmarks.list', [
        { title: 'Empty', url: '', faviconUrl: '', createdAt: 'today' },
      ]),
    ).toThrow(/bookmarks\.list.*url/);
  });

  it('accepts null settings values', () => {
    expect(validateBridgeResponse('settings.get', null)).toBeNull();
  });

  it('rejects non-string settings values', () => {
    expect(() => validateBridgeResponse('settings.get', 42)).toThrow(
      /settings\.get/,
    );
  });

  it('rejects non-boolean command results', () => {
    expect(() => validateBridgeResponse('tabs.close', 'yes')).toThrow(
      /tabs\.close.*boolean/,
    );
  });

  it('rejects commands with empty ids', () => {
    expect(() =>
      validateBridgeResponse('commands.list', [
        { id: '', title: 'Bad', category: '', shortcut: '' },
      ]),
    ).toThrow(/commands\.list.*id/);
  });

  it('rejects windows with empty ids', () => {
    expect(() =>
      validateBridgeResponse('windows.list', [
        { id: '', activeTabId: null, isPrivate: false, tabIds: [] },
      ]),
    ).toThrow(/windows\.list.*id/);
  });
});

describe('event validation and listener isolation', () => {
  it('accepts a valid tab.created event', () => {
    expect(validateBridgeEvent('tab.created', tab)).toEqual(tab);
  });

  it('rejects invalid tab.updated numeric fields', () => {
    expect(() =>
      validateBridgeEvent('tab.updated', {
        tabId: 'tab-1',
        zoomLevel: Number.NaN,
      }),
    ).toThrow(/tab\.updated.*zoomLevel/);
  });

  it('rejects negative tab move indexes', () => {
    expect(() =>
      validateBridgeEvent('tab.moved', {
        tabId: 'tab-1',
        fromWindowId: 'window-1',
        toWindowId: 'window-2',
        toIndex: -1,
      }),
    ).toThrow(/tab\.moved.*toIndex/);
  });

  it('rejects malformed setting.changed payloads', () => {
    expect(() =>
      validateBridgeEvent('setting.changed', { key: 'theme', value: 1 }),
    ).toThrow(/setting\.changed.*value/);
  });

  it('clamps download.changed percentages', () => {
    expect(validateBridgeEvent('download.changed', { percent: -20 })).toEqual({
      percent: 0,
    });
  });

  it('rejects payloads for void events', () => {
    expect(() =>
      validateBridgeEvent('app.stateChanged', { unexpected: true }),
    ).toThrow(/app\.stateChanged/);
  });

  it('normalizes native empty-object payloads for void events', () => {
    expect(validateBridgeEvent('app.stateChanged', {})).toBeUndefined();
  });

  it('rejects unsupported event names with their name', () => {
    expect(() =>
      validateBridgeEvent(
        'unknown.event' as keyof import('../bridge/fubuki').EventMap,
        null,
      ),
    ).toThrow(/unknown\.event.*unsupported/);
  });

  it('continues notifying listeners after one throws', () => {
    const later = vi.fn();
    const reportError = vi.fn();
    notifyBridgeListeners(
      'tab.created',
      [
        () => {
          throw new Error('broken listener');
        },
        later,
      ],
      tab,
      reportError,
    );
    expect(reportError).toHaveBeenCalledOnce();
    expect(later).toHaveBeenCalledWith(tab);
  });
});
