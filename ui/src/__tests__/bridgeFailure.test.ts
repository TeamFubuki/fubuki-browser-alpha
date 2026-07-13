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
