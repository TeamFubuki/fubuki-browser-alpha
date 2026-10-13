import { describe, expect, it } from 'vitest';
import { tabDropEdge, tabDropIndex } from '../components/tabDrag';
import type { Tab } from '../bridge/fubuki';

const tab = (id: string, isPinned = false, windowId = 'w1'): Tab => ({
  id,
  windowId,
  isPinned,
  title: id,
  url: '',
  faviconUrl: '',
  errorText: '',
  zoomLevel: 0,
  isLoading: false,
  canGoBack: false,
  canGoForward: false,
  isActive: false,
});

function movedIds(
  all: Tab[],
  source: string,
  target: string,
  edge: 'before' | 'after',
) {
  const index = tabDropIndex(all, source, { tabId: target, edge });
  const result = all.map((item) => item.id);
  if (index !== null) {
    result.splice(result.indexOf(source), 1);
    result.splice(index, 0, source);
  }
  return result;
}

describe('tab drag insertion', () => {
  const all = [tab('a'), tab('b'), tab('c'), tab('d')];

  it('inserts before and after a target while moving down', () => {
    expect(movedIds(all, 'a', 'c', 'before')).toEqual(['b', 'a', 'c', 'd']);
    expect(movedIds(all, 'a', 'c', 'after')).toEqual(['b', 'c', 'a', 'd']);
    expect(movedIds(all, 'a', 'd', 'after')).toEqual(['b', 'c', 'd', 'a']);
  });

  it('inserts before and after a target while moving up', () => {
    expect(movedIds(all, 'd', 'b', 'before')).toEqual(['a', 'd', 'b', 'c']);
    expect(movedIds(all, 'd', 'b', 'after')).toEqual(['a', 'b', 'd', 'c']);
    expect(movedIds(all, 'd', 'a', 'before')).toEqual(['d', 'a', 'b', 'c']);
  });

  it('does not send moves for unchanged positions', () => {
    expect(tabDropIndex(all, 'a', { tabId: 'a', edge: 'after' })).toBeNull();
    expect(tabDropIndex(all, 'a', { tabId: 'b', edge: 'before' })).toBeNull();
    expect(tabDropIndex(all, 'b', { tabId: 'a', edge: 'after' })).toBeNull();
  });

  it('rejects unknown or closed tabs and tabs from another window', () => {
    expect(
      tabDropIndex(all, 'missing', { tabId: 'b', edge: 'after' }),
    ).toBeNull();
    expect(
      tabDropIndex(all, 'a', { tabId: 'missing', edge: 'before' }),
    ).toBeNull();
    expect(
      tabDropIndex([tab('a'), tab('b', false, 'w2')], 'a', {
        tabId: 'b',
        edge: 'after',
      }),
    ).toBeNull();
  });

  it('reorders pinned tabs without crossing the pinned boundary', () => {
    const mixed = [tab('p1', true), tab('p2', true), ...all];
    expect(movedIds(mixed, 'p1', 'p2', 'after')).toEqual([
      'p2',
      'p1',
      'a',
      'b',
      'c',
      'd',
    ]);
    expect(tabDropIndex(mixed, 'p1', { tabId: 'a', edge: 'after' })).toBeNull();
    expect(
      tabDropIndex(mixed, 'a', { tabId: 'p1', edge: 'before' }),
    ).toBeNull();
  });

  it('uses full tab indexes when the target is shown in a search result', () => {
    expect(movedIds(all, 'd', 'b', 'before')).toEqual(['a', 'd', 'b', 'c']);
    const mixed = [tab('p1', true), ...all];
    expect(tabDropIndex(mixed, 'd', { tabId: 'b', edge: 'before' })).toBe(2);
  });
});

describe('tab drop edge', () => {
  const rect = { left: 10, top: 20, width: 28, height: 32 };
  it('splits normal rows vertically', () => {
    expect(tabDropEdge({ clientX: 30, clientY: 35 }, rect, false)).toBe(
      'before',
    );
    expect(tabDropEdge({ clientX: 10, clientY: 36 }, rect, false)).toBe(
      'after',
    );
  });
  it('splits pinned icons horizontally', () => {
    expect(tabDropEdge({ clientX: 23, clientY: 50 }, rect, true)).toBe(
      'before',
    );
    expect(tabDropEdge({ clientX: 24, clientY: 20 }, rect, true)).toBe('after');
  });
});
