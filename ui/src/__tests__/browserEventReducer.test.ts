import { describe, expect, it } from 'vitest';
import {
  reduceBrowserEvent,
  reduceTabClosed,
  type BrowserEvent,
  type BrowserEventState,
} from '../stores/browserEventReducer';
import type { FrostTabState, FrostWindowState, Tab } from '../bridge/fubuki';

const tab = (id: string, active = false, windowId = 'window-1'): Tab => ({
  id,
  windowId,
  title: id,
  url: `https://${id}.test`,
  faviconUrl: '',
  errorText: '',
  zoomLevel: 0,
  isLoading: false,
  canGoBack: false,
  canGoForward: false,
  isActive: active,
  isPinned: false,
});

const frostTab = (
  id: string,
  active = false,
  windowId = 'window-1',
): FrostTabState => tab(id, active, windowId);

const windowState = (
  id: string,
  activeTabId: string | null = null,
): FrostWindowState => ({
  id,
  activeTabId,
  isPrivate: false,
  tabIds: [],
});

const stateWith = (
  tabs: Tab[] = [tab('tab-1', true), tab('tab-2'), tab('tab-3')],
): BrowserEventState => ({
  windowId: 'window-1',
  activeTabId: tabs.find((item) => item.isActive)?.id ?? '',
  tabs,
  windows: [
    { id: 'window-1', activeTabId: 'tab-1', tabs: [] },
    { id: 'window-2', activeTabId: 'other-tab', tabs: [] },
  ],
});

const reduce = (state: BrowserEventState, event: BrowserEvent) =>
  reduceBrowserEvent(state, event);

describe('browser event reducer: tab.created', () => {
  it('adds a local tab without changing the input', () => {
    const state = stateWith();
    const result = reduce(state, {
      type: 'tab.created',
      payload: frostTab('tab-4'),
    });

    expect(result.changed).toBe(true);
    expect(result.state.tabs.map((item) => item.id)).toEqual([
      'tab-1',
      'tab-2',
      'tab-3',
      'tab-4',
    ]);
    expect(state.tabs).toHaveLength(3);
  });

  it('activates a created tab and clears the previous active flag', () => {
    const result = reduce(stateWith(), {
      type: 'tab.created',
      payload: frostTab('tab-4', true),
    });

    expect(result.state.activeTabId).toBe('tab-4');
    expect(
      result.state.tabs.filter((item) => item.isActive).map((item) => item.id),
    ).toEqual(['tab-4']);
  });

  it('ignores a tab created for another window', () => {
    const state = stateWith();
    const result = reduce(state, {
      type: 'tab.created',
      payload: frostTab('foreign-tab', false, 'window-2'),
    });

    expect(result).toMatchObject({ changed: false, refreshSnapshot: false });
    expect(result.state).toBe(state);
  });

  it('is idempotent for an exact duplicate create event', () => {
    const state = stateWith([...stateWith().tabs, tab('tab-4')]);
    const result = reduce(state, {
      type: 'tab.created',
      payload: frostTab('tab-4'),
    });

    expect(result.state).toBe(state);
    expect(result.changed).toBe(false);
  });

  it('does not overwrite an existing tab on a conflicting duplicate create', () => {
    const state = stateWith();
    const result = reduce(state, {
      type: 'tab.created',
      payload: { ...frostTab('tab-2'), title: 'stale create' },
    });

    expect(result.state).toBe(state);
    expect(result.state.tabs[1].title).toBe('tab-2');
  });
});

describe('browser event reducer: tab.updated', () => {
  it('applies supported fields to a known local tab', () => {
    const result = reduce(stateWith(), {
      type: 'tab.updated',
      payload: {
        tabId: 'tab-2',
        title: 'Updated title',
        url: 'https://updated.test',
        isLoading: true,
        isPinned: true,
      },
    });

    expect(result.changed).toBe(true);
    expect(result.state.tabs[1]).toMatchObject({
      title: 'Updated title',
      url: 'https://updated.test',
      isLoading: true,
      isPinned: true,
    });
  });

  it('ignores an update containing no supported fields', () => {
    const state = stateWith();
    const result = reduce(state, {
      type: 'tab.updated',
      payload: { tabId: 'tab-2' },
    });

    expect(result.state).toBe(state);
    expect(result.changed).toBe(false);
  });

  it('ignores an update for an unknown tab', () => {
    const state = stateWith();
    const result = reduce(state, {
      type: 'tab.updated',
      payload: { tabId: 'missing-tab', title: 'unknown' },
    });

    expect(result.state).toBe(state);
    expect(result.refreshSnapshot).toBe(false);
  });

  it('requests a snapshot when a known tab reports another window', () => {
    const result = reduce(stateWith(), {
      type: 'tab.updated',
      payload: { tabId: 'tab-2', windowId: 'window-2', title: 'moved' },
    });

    expect(result).toMatchObject({ changed: false, refreshSnapshot: true });
  });
});

describe('browser event reducer: tab.closed', () => {
  it('selects the tab immediately to the left after closing the active middle tab', () => {
    const state = stateWith([tab('tab-1'), tab('tab-2', true), tab('tab-3')]);
    const result = reduceTabClosed(state, { tabId: 'tab-2' });

    expect(result.state.activeTabId).toBe('tab-1');
    expect(
      result.state.tabs.find((item) => item.id === 'tab-1')?.isActive,
    ).toBe(true);
  });

  it('selects the new first tab after closing the first active tab', () => {
    const result = reduceTabClosed(stateWith(), { tabId: 'tab-1' });

    expect(result.state.activeTabId).toBe('tab-2');
    expect(
      result.state.tabs.find((item) => item.id === 'tab-2')?.isActive,
    ).toBe(true);
  });

  it('selects the previous tab after closing the last active tab', () => {
    const state = stateWith([tab('tab-1'), tab('tab-2', true)]);
    const result = reduceTabClosed(state, { tabId: 'tab-2' });

    expect(result.state.activeTabId).toBe('tab-1');
  });

  it('clears the active tab when the final tab is closed', () => {
    const result = reduceTabClosed(stateWith([tab('tab-1', true)]), {
      tabId: 'tab-1',
    });

    expect(result.state.tabs).toEqual([]);
    expect(result.state.activeTabId).toBe('');
  });

  it('keeps the active tab when a non-active tab is closed', () => {
    const result = reduceTabClosed(stateWith(), { tabId: 'tab-3' });

    expect(result.state.activeTabId).toBe('tab-1');
    expect(result.state.tabs.map((item) => item.id)).toEqual([
      'tab-1',
      'tab-2',
    ]);
  });

  it('ignores a duplicate close for an unknown tab', () => {
    const state = stateWith();
    const result = reduceTabClosed(state, { tabId: 'missing-tab' });

    expect(result.state).toBe(state);
    expect(result).toMatchObject({ changed: false, refreshSnapshot: false });
  });
});

describe('browser event reducer: tab.activated', () => {
  it('switches the active tab and normalizes all flags', () => {
    const result = reduce(stateWith(), {
      type: 'tab.activated',
      payload: { tabId: 'tab-3' },
    });

    expect(result.state.activeTabId).toBe('tab-3');
    expect(result.state.tabs.filter((item) => item.isActive)).toHaveLength(1);
    expect(result.state.tabs[2].isActive).toBe(true);
  });

  it('is idempotent for an activation that is already applied', () => {
    const state = stateWith();
    const result = reduce(state, {
      type: 'tab.activated',
      payload: { tabId: 'tab-1' },
    });

    expect(result.state).toBe(state);
    expect(result.changed).toBe(false);
  });

  it('ignores activation for an unknown or foreign tab', () => {
    const state = stateWith();
    const result = reduce(state, {
      type: 'tab.activated',
      payload: { tabId: 'foreign-tab' },
    });

    expect(result.state).toBe(state);
    expect(result.refreshSnapshot).toBe(false);
  });
});

describe('browser event reducer: tab.moved', () => {
  it('reorders a local tab within the current window', () => {
    const result = reduce(stateWith(), {
      type: 'tab.moved',
      payload: {
        tabId: 'tab-1',
        fromWindowId: 'window-1',
        toWindowId: 'window-1',
        toIndex: 2,
      },
    });

    expect(result.state.tabs.map((item) => item.id)).toEqual([
      'tab-2',
      'tab-3',
      'tab-1',
    ]);
  });

  it('does not allocate a change when a tab is already at the target index', () => {
    const state = stateWith();
    const result = reduce(state, {
      type: 'tab.moved',
      payload: {
        tabId: 'tab-2',
        fromWindowId: 'window-1',
        toWindowId: 'window-1',
        toIndex: 1,
      },
    });

    expect(result.state).toBe(state);
    expect(result.changed).toBe(false);
  });

  it('clamps an oversized target index to the last tab', () => {
    const result = reduce(stateWith(), {
      type: 'tab.moved',
      payload: {
        tabId: 'tab-1',
        fromWindowId: 'window-1',
        toWindowId: 'window-1',
        toIndex: 99,
      },
    });

    expect(result.state.tabs.at(-1)?.id).toBe('tab-1');
  });

  it('requests a snapshot for an invalid target index', () => {
    const result = reduce(stateWith(), {
      type: 'tab.moved',
      payload: {
        tabId: 'tab-1',
        fromWindowId: 'window-1',
        toWindowId: 'window-1',
        toIndex: -1,
      },
    });

    expect(result).toMatchObject({ changed: false, refreshSnapshot: true });
  });

  it('requests a snapshot for an unknown destination tab', () => {
    const result = reduce(stateWith(), {
      type: 'tab.moved',
      payload: {
        tabId: 'missing-tab',
        fromWindowId: 'window-2',
        toWindowId: 'window-1',
        toIndex: 0,
      },
    });

    expect(result).toMatchObject({ changed: false, refreshSnapshot: true });
  });

  it('removes a tab moved out of the current window', () => {
    const result = reduce(stateWith(), {
      type: 'tab.moved',
      payload: {
        tabId: 'tab-1',
        fromWindowId: 'window-1',
        toWindowId: 'window-2',
        toIndex: 0,
      },
    });

    expect(result.state.tabs.map((item) => item.id)).toEqual([
      'tab-2',
      'tab-3',
    ]);
    expect(result.state.activeTabId).toBe('tab-2');
  });

  it('does not refresh for an unrelated cross-window move', () => {
    const state = stateWith();
    const result = reduce(state, {
      type: 'tab.moved',
      payload: {
        tabId: 'foreign-tab',
        fromWindowId: 'window-2',
        toWindowId: 'window-3',
        toIndex: 0,
      },
    });

    expect(result.state).toBe(state);
    expect(result.refreshSnapshot).toBe(false);
  });

  it('requests a snapshot when a tab enters the current window', () => {
    const result = reduce(stateWith(), {
      type: 'tab.moved',
      payload: {
        tabId: 'foreign-tab',
        fromWindowId: 'window-2',
        toWindowId: 'window-1',
        toIndex: 0,
      },
    });

    expect(result.refreshSnapshot).toBe(true);
    expect(result.state.tabs).toHaveLength(3);
  });
});

describe('browser event reducer: window events', () => {
  it('adds a newly created window with the event metadata', () => {
    const result = reduce(stateWith(), {
      type: 'window.created',
      payload: windowState('window-3', 'new-tab'),
    });

    expect(result.state.windows.at(-1)).toMatchObject({
      id: 'window-3',
      activeTabId: 'new-tab',
      private: false,
    });
  });

  it('ignores an exact duplicate window create', () => {
    const state = stateWith();
    const result = reduce(state, {
      type: 'window.created',
      payload: windowState('window-2', 'other-tab'),
    });

    expect(result.state).toBe(state);
    expect(result.changed).toBe(false);
  });

  it('refreshes on a conflicting duplicate window create', () => {
    const result = reduce(stateWith(), {
      type: 'window.created',
      payload: { ...windowState('window-2'), isPrivate: true },
    });

    expect(result).toMatchObject({ changed: false, refreshSnapshot: true });
  });

  it('refreshes when window.created has no payload', () => {
    const result = reduce(stateWith(), {
      type: 'window.created',
      payload: undefined,
    });

    expect(result.refreshSnapshot).toBe(true);
  });

  it('removes a known non-current window on close', () => {
    const result = reduce(stateWith(), {
      type: 'window.closed',
      payload: { windowId: 'window-2' },
    });

    expect(result.state.windows.map((item) => item.id)).toEqual(['window-1']);
    expect(result.changed).toBe(true);
  });

  it('ignores a duplicate close for an unknown window', () => {
    const state = stateWith();
    const result = reduce(state, {
      type: 'window.closed',
      payload: { windowId: 'missing-window' },
    });

    expect(result.state).toBe(state);
    expect(result.refreshSnapshot).toBe(false);
  });

  it('refreshes when the current window closes', () => {
    const result = reduce(stateWith(), {
      type: 'window.closed',
      payload: { windowId: 'window-1' },
    });

    expect(result).toMatchObject({ changed: false, refreshSnapshot: true });
  });

  it('refreshes when window.closed has no payload', () => {
    const result = reduce(stateWith(), {
      type: 'window.closed',
      payload: undefined,
    });

    expect(result.refreshSnapshot).toBe(true);
  });

  it('refreshes when the current window is focused', () => {
    const result = reduce(stateWith(), {
      type: 'window.focused',
      payload: { windowId: 'window-1' },
    });

    expect(result).toMatchObject({ changed: false, refreshSnapshot: true });
  });

  it('ignores focus events for another window', () => {
    const state = stateWith();
    const result = reduce(state, {
      type: 'window.focused',
      payload: { windowId: 'window-2' },
    });

    expect(result.state).toBe(state);
    expect(result.refreshSnapshot).toBe(false);
  });

  it('refreshes when window.focused has no payload', () => {
    const result = reduce(stateWith(), {
      type: 'window.focused',
      payload: undefined,
    });

    expect(result.refreshSnapshot).toBe(true);
  });
});

describe('browser event reducer: immutability', () => {
  it('does not mutate nested tab or window data', () => {
    const state = stateWith();
    const before = structuredClone(state);

    reduce(state, {
      type: 'tab.updated',
      payload: { tabId: 'tab-2', title: 'new title' },
    });
    reduce(state, {
      type: 'tab.moved',
      payload: {
        tabId: 'tab-1',
        fromWindowId: 'window-1',
        toWindowId: 'window-1',
        toIndex: 2,
      },
    });

    expect(state).toEqual(before);
  });
});
