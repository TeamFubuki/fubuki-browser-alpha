import { describe, expect, it } from 'vitest';
import { adjacentTabId, tabIdForNumberShortcut } from '../tabShortcuts';

const tabs = [{ id: 'one' }, { id: 'two' }, { id: 'three' }];

describe('tab number shortcuts', () => {
  it('maps 1 through 8 by position', () => {
    expect(tabIdForNumberShortcut(tabs, '1')).toBe('one');
    expect(tabIdForNumberShortcut(tabs, '3')).toBe('three');
    expect(tabIdForNumberShortcut(tabs, '4')).toBeUndefined();
  });

  it('maps 9 to the last tab', () => {
    expect(tabIdForNumberShortcut(tabs, '9')).toBe('three');
  });
});

describe('adjacent tab shortcuts', () => {
  it('cycles forward and wraps', () => {
    expect(adjacentTabId(tabs, 'one', true)).toBe('two');
    expect(adjacentTabId(tabs, 'three', true)).toBe('one');
  });

  it('cycles backward and wraps', () => {
    expect(adjacentTabId(tabs, 'one', false)).toBe('three');
    expect(adjacentTabId(tabs, 'two', false)).toBe('one');
  });

  it('handles missing and empty state safely', () => {
    expect(adjacentTabId(tabs, 'missing', true)).toBe('two');
    expect(adjacentTabId([], 'missing', true)).toBeUndefined();
  });
});
