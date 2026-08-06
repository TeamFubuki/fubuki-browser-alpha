import { describe, expect, it } from 'vitest';
import {
  focusAfterClose,
  focusTargetAfterClose,
  navigationTarget,
  reorderTargetIndex,
  tabIndexFor,
} from '../components/verticalTabKeyboard';
import type { Tab } from '../bridge/fubuki';

const tab = (id: string, isPinned = false): Tab => ({
  id,
  windowId: 'window-1',
  title: id,
  url: `https://${id}.test`,
  faviconUrl: '',
  errorText: '',
  zoomLevel: 0,
  isLoading: false,
  canGoBack: false,
  canGoForward: false,
  isActive: false,
  isPinned,
});

describe('vertical tab keyboard navigation', () => {
  it('moves down and wraps', () => {
    expect(navigationTarget(['a', 'b', 'c'], 'b', 'ArrowDown')).toBe('c');
    expect(navigationTarget(['a', 'b', 'c'], 'c', 'ArrowDown')).toBe('a');
  });

  it('moves up and wraps', () => {
    expect(navigationTarget(['a', 'b', 'c'], 'b', 'ArrowUp')).toBe('a');
    expect(navigationTarget(['a', 'b', 'c'], 'a', 'ArrowUp')).toBe('c');
  });

  it('supports Home and End', () => {
    expect(navigationTarget(['a', 'b', 'c'], 'b', 'Home')).toBe('a');
    expect(navigationTarget(['a', 'b', 'c'], 'b', 'End')).toBe('c');
  });

  it('ignores an unknown focused tab', () => {
    expect(navigationTarget(['a', 'b'], 'missing', 'ArrowDown')).toBeNull();
  });

  it('keeps one roving tab stop for a focused list', () => {
    expect(tabIndexFor('b', ['a', 'b'], 'a', 'b')).toBe(0);
    expect(tabIndexFor('a', ['a', 'b'], 'a', 'b')).toBe(-1);
  });

  it('falls back to the active tab when focus left the list', () => {
    expect(tabIndexFor('b', ['a', 'b'], 'b', 'missing')).toBe(0);
    expect(tabIndexFor('a', ['a', 'b'], 'b', 'missing')).toBe(-1);
  });

  it('falls back to the first visible tab without an active tab', () => {
    expect(tabIndexFor('a', ['a', 'b'], 'missing', 'missing')).toBe(0);
    expect(tabIndexFor('b', ['a', 'b'], 'missing', 'missing')).toBe(-1);
  });

  it('returns no tab stop for an empty list', () => {
    expect(tabIndexFor('a', [], 'a', 'a')).toBe(-1);
  });

  it('restores focus to the first tab after closing the first tab', () => {
    expect(focusAfterClose(['a', 'b', 'c'], 0)).toBe('b');
  });

  it('restores focus to the left neighbor after closing a middle tab', () => {
    expect(focusAfterClose(['a', 'b', 'c'], 1)).toBe('a');
  });

  it('restores focus to the left neighbor after closing the last tab', () => {
    expect(focusAfterClose(['a', 'b', 'c'], 2)).toBe('b');
  });

  it('returns null when the last tab is closed', () => {
    expect(focusAfterClose(['a'], 0)).toBeNull();
  });

  it('falls back to the new active tab after closing the last tab', () => {
    expect(focusTargetAfterClose(null, 'replacement')).toBe('replacement');
  });

  it('moves a normal tab only across normal tabs', () => {
    const all = [tab('pinned', true), tab('a'), tab('b'), tab('c')];
    expect(reorderTargetIndex(all, all.slice(1), 'b', 'ArrowDown')).toBe(3);
    expect(reorderTargetIndex(all, all.slice(1), 'b', 'ArrowUp')).toBe(1);
  });

  it('moves a pinned tab only across pinned tabs', () => {
    const all = [tab('p1', true), tab('p2', true), tab('a')];
    expect(reorderTargetIndex(all, all.slice(0, 2), 'p1', 'ArrowDown')).toBe(1);
    expect(
      reorderTargetIndex(all, all.slice(0, 2), 'p2', 'ArrowDown'),
    ).toBeNull();
  });

  it('does not move beyond the visible list', () => {
    const all = [tab('a'), tab('b')];
    expect(reorderTargetIndex(all, all, 'a', 'ArrowUp')).toBeNull();
    expect(reorderTargetIndex(all, all, 'b', 'ArrowDown')).toBeNull();
  });

  it('does not reorder an unknown tab', () => {
    const all = [tab('a'), tab('b')];
    expect(reorderTargetIndex(all, all, 'missing', 'ArrowDown')).toBeNull();
  });
});
