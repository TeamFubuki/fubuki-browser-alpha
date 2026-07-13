import { describe, expect, it } from 'vitest';

import { parseBridgeResponse } from '../bridge/response';

describe('parseBridgeResponse', () => {
  it('returns successful bridge values', () => {
    expect(parseBridgeResponse<boolean>('true')).toBe(true);
    expect(parseBridgeResponse<{ id: string }>(`{"id":"tab-1"}`)).toEqual({
      id: 'tab-1',
    });
  });

  it('throws instead of treating a failure envelope as a result', () => {
    expect(() =>
      parseBridgeResponse('{"ok":false,"error":"host failed"}'),
    ).toThrow('host failed');
  });

  it('throws for malformed JSON', () => {
    expect(() => parseBridgeResponse('not json')).toThrow();
  });
});
