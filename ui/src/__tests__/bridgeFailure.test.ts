import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

type Query = {
  request: string;
  onSuccess: (response: string) => void;
  onFailure: (code: number, message: string) => void;
};

const originalWindow = Object.getOwnPropertyDescriptor(globalThis, 'window');

beforeEach(() => {
  vi.resetModules();
});

afterEach(() => {
  vi.restoreAllMocks();
  if (originalWindow) {
    Object.defineProperty(globalThis, 'window', originalWindow);
  } else {
    Reflect.deleteProperty(globalThis, 'window');
  }
});

function installNativeBridge(handler: (query: Query) => void) {
  Object.defineProperty(globalThis, 'window', {
    configurable: true,
    value: {
      addEventListener: vi.fn(),
      cefQuery: handler,
    },
  });
}

describe('Frost bridge failures', () => {
  it('rejects the originating promise and exposes callback failure details', async () => {
    installNativeBridge((query) => query.onFailure(502, 'Host timed out'));
    const { BridgeError, invokeBridge, onBridgeFailure } =
      await import('../bridge/fubuki');
    const listener = vi.fn();
    onBridgeFailure(listener);

    await expect(invokeBridge('tabs.create', { active: true })).rejects.toEqual(
      expect.objectContaining({
        name: 'BridgeError',
        method: 'tabs.create',
        code: 502,
        message: 'Host timed out',
      }),
    );
    expect(listener).toHaveBeenCalledWith({
      method: 'tabs.create',
      error: expect.any(BridgeError),
    });
  });

  it('rejects malformed native responses instead of treating them as success', async () => {
    installNativeBridge((query) => query.onSuccess('{not json'));
    const { invokeBridge } = await import('../bridge/fubuki');

    await expect(invokeBridge('tabs.list')).rejects.toThrow(
      'Invalid bridge response',
    );
  });

  it('rejects native error envelopes even when delivered through onSuccess', async () => {
    installNativeBridge((query) =>
      query.onSuccess(
        JSON.stringify({ ok: false, error: 'FrostEngine rejected request' }),
      ),
    );
    const { invokeBridge } = await import('../bridge/fubuki');

    await expect(invokeBridge('app.snapshot')).rejects.toEqual(
      expect.objectContaining({
        name: 'BridgeError',
        method: 'app.snapshot',
        message: 'FrostEngine rejected request',
      }),
    );
  });

  it('settles a failed request even when a failure listener throws', async () => {
    installNativeBridge((query) =>
      queueMicrotask(() => query.onFailure(500, 'Native operation failed')),
    );
    const consoleError = vi
      .spyOn(console, 'error')
      .mockImplementation(() => undefined);
    const { invokeBridge, onBridgeFailure } = await import('../bridge/fubuki');
    const survivingListener = vi.fn();
    onBridgeFailure(() => {
      throw new Error('Listener failed');
    });
    onBridgeFailure(survivingListener);

    await expect(invokeBridge('tabs.list')).rejects.toThrow(
      'Native operation failed',
    );
    expect(survivingListener).toHaveBeenCalledOnce();
    expect(consoleError).toHaveBeenCalledOnce();
  });

  it('rejects an explicitly unhandled command', async () => {
    installNativeBridge((query) =>
      query.onSuccess(JSON.stringify({ handled: false, id: 'windows.create' })),
    );
    const { commands } = await import('../bridge/fubuki');

    await expect(commands.execute('windows.create')).rejects.toThrow(
      'Command was not handled: windows.create',
    );
  });

  it('treats a false mutation result as a failure', async () => {
    installNativeBridge((query) => query.onSuccess('false'));
    const { tabs } = await import('../bridge/fubuki');

    await expect(tabs.close('tab-1')).rejects.toThrow(
      'Operation was not completed',
    );
  });
});
