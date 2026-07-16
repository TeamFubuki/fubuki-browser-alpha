import { describe, expect, it } from 'vitest';
import { recordTimestamp } from '../pages/internal/data';

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
