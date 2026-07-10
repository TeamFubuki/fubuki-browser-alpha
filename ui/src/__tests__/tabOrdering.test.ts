import { describe, expect, it } from 'vitest';
import { reorderTabById } from '../tabOrdering';

const tabs = [{ id: 'a' }, { id: 'b' }, { id: 'c' }];

describe('reorderTabById', () => {
  it('moves a tab one position forward without skipping a slot', () => {
    expect(reorderTabById(tabs, 'a', 1).map((tab) => tab.id)).toEqual([
      'b',
      'a',
      'c',
    ]);
  });

  it('moves a tab backward', () => {
    expect(reorderTabById(tabs, 'c', 0).map((tab) => tab.id)).toEqual([
      'c',
      'a',
      'b',
    ]);
  });

  it('does not mutate the source list', () => {
    reorderTabById(tabs, 'a', 2);
    expect(tabs.map((tab) => tab.id)).toEqual(['a', 'b', 'c']);
  });
});
