import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  INTERNAL_DATA_CHANGED_EVENT,
  invokeInternalAction,
} from '../pages/internal/actions';

const originalWindow = globalThis.window;

afterEach(() => {
  Object.defineProperty(globalThis, 'window', {
    configurable: true,
    value: originalWindow,
    writable: true,
  });
});

describe('internal page action channel', () => {
  it('sends a scoped action and announces that page data changed', async () => {
    const dispatchEvent = vi.fn();
    const cefQuery = vi.fn(
      (query: { request: string; onSuccess: (response: string) => void }) =>
        query.onSuccess('{"ok":true}'),
    );
    Object.defineProperty(globalThis, 'window', {
      configurable: true,
      value: {
        cefQuery,
        dispatchEvent,
        fubukiInternalMarker: true,
        scrollX: 12,
        scrollY: 340,
      },
      writable: true,
    });

    await invokeInternalAction('appearance', 'dark');

    expect(cefQuery).toHaveBeenCalledOnce();
    const request = JSON.parse(cefQuery.mock.calls[0][0].request) as Record<
      string,
      string
    >;
    expect(request).toEqual({
      channel: 'internal.action',
      key: 'appearance',
      value: 'dark',
    });
    expect(dispatchEvent).toHaveBeenCalledOnce();
    expect(dispatchEvent.mock.calls[0][0].type).toBe(
      INTERNAL_DATA_CHANGED_EVENT,
    );
  });

  it('surfaces native action failures', async () => {
    Object.defineProperty(globalThis, 'window', {
      configurable: true,
      value: {
        cefQuery: (query: {
          onFailure: (code: number, message: string) => void;
        }) => query.onFailure(403, 'Action denied'),
        dispatchEvent: vi.fn(),
        fubukiInternalMarker: true,
        scrollX: 0,
        scrollY: 0,
      },
      writable: true,
    });

    await expect(invokeInternalAction('openDevTools', '1')).rejects.toThrow(
      'Action denied',
    );
  });
});
