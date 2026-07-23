import { afterEach, describe, expect, it, vi } from 'vitest';
import { loadInternalData, recordTimestamp } from '../pages/internal/data';
import { matchesSearchTerms } from '../pages/internal/settingsSearch';

const originalWindow = globalThis.window;
const originalFetch = globalThis.fetch;

afterEach(() => {
  Object.defineProperty(globalThis, 'window', {
    configurable: true,
    value: originalWindow,
    writable: true,
  });
  globalThis.fetch = originalFetch;
});

describe('internal page record timestamps', () => {
  it('accepts FrostStore Unix timestamps in seconds', () => {
    expect(recordTimestamp('1710000000')?.getTime()).toBe(1_710_000_000_000);
  });

  it('accepts ISO timestamps and rejects invalid values', () => {
    expect(recordTimestamp('2026-07-16T00:00:00Z')?.toISOString()).toBe(
      '2026-07-16T00:00:00.000Z',
    );
    expect(recordTimestamp('not-a-date')).toBeUndefined();
  });
});

describe('internal settings search', () => {
  it('matches localized control labels case-insensitively', () => {
    const terms = ['Search', 'Google', 'DuckDuckGo', 'カスタム'];
    expect(matchesSearchTerms('google', terms)).toBe(true);
    expect(matchesSearchTerms('カスタム', terms)).toBe(true);
    expect(matchesSearchTerms('privacy', terms)).toBe(false);
    expect(matchesSearchTerms('   ', terms)).toBe(true);
  });
});

describe('internal page data errors', () => {
  it('rejects a non-200 store projection so the page shows loadError', async () => {
    Object.defineProperty(globalThis, 'window', {
      configurable: true,
      value: {
        location: {
          href: 'fubuki://bookmarks/',
          hostname: 'bookmarks',
          search: '',
        },
      },
      writable: true,
    });
    globalThis.fetch = vi.fn().mockResolvedValue({ ok: false, status: 503 });

    await expect(loadInternalData()).rejects.toThrow(
      'Internal data request failed (503)',
    );
  });
});
