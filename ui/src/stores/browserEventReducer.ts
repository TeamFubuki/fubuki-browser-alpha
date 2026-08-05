import {
  type BrowserState,
  type FrostTabState,
  type FrostWindowState,
  type Tab,
  type WindowSnapshot,
} from '../bridge/fubuki';

/** The slices of browser state that tab/window events can update. */
export type BrowserEventState = Pick<
  BrowserState,
  'windowId' | 'activeTabId' | 'tabs' | 'windows'
>;

export type BrowserEvent =
  | { type: 'tab.created'; payload: FrostTabState }
  | {
      type: 'tab.updated';
      payload: Partial<FrostTabState> & { tabId: string };
    }
  | { type: 'tab.closed'; payload: { tabId: string } }
  | { type: 'tab.activated'; payload: { tabId: string } }
  | {
      type: 'tab.moved';
      payload: {
        tabId: string;
        fromWindowId: string;
        toWindowId: string;
        toIndex: number;
      };
    }
  | { type: 'window.created'; payload: FrostWindowState | void }
  | { type: 'window.closed'; payload: { windowId: string } | void }
  | { type: 'window.focused'; payload: { windowId: string } | void };

export type BrowserEventResult = {
  state: BrowserEventState;
  changed: boolean;
  refreshSnapshot: boolean;
};

const unchanged = (state: BrowserEventState): BrowserEventResult => ({
  state,
  changed: false,
  refreshSnapshot: false,
});

const changed = (
  state: BrowserEventState,
  refreshSnapshot = false,
): BrowserEventResult => ({
  state,
  changed: true,
  refreshSnapshot,
});

const refresh = (state: BrowserEventState): BrowserEventResult => ({
  state,
  changed: false,
  refreshSnapshot: true,
});

export function reduceBrowserEvent(
  state: BrowserEventState,
  event: BrowserEvent,
): BrowserEventResult {
  switch (event.type) {
    case 'tab.created':
      return reduceTabCreated(state, event.payload);
    case 'tab.updated':
      return reduceTabUpdated(state, event.payload);
    case 'tab.closed':
      return reduceTabClosed(state, event.payload);
    case 'tab.activated':
      return reduceTabActivated(state, event.payload);
    case 'tab.moved':
      return reduceTabMoved(state, event.payload);
    case 'window.created':
      return reduceWindowCreated(state, event.payload);
    case 'window.closed':
      return reduceWindowClosed(state, event.payload);
    case 'window.focused':
      return reduceWindowFocused(state, event.payload);
  }
}

export function reduceTabCreated(
  state: BrowserEventState,
  nextFrostTab: FrostTabState,
): BrowserEventResult {
  if (nextFrostTab.windowId !== state.windowId) return unchanged(state);

  const nextTab = fromFrostTab(nextFrostTab);
  const existingIndex = state.tabs.findIndex((tab) => tab.id === nextTab.id);
  if (existingIndex >= 0) {
    const existingTab = state.tabs[existingIndex];
    // A duplicate create is deliberately idempotent. Replacing it could
    // overwrite a newer local update with an older event payload.
    if (sameTab(existingTab, nextTab)) return unchanged(state);
    return unchanged(state);
  }

  const tabs = nextTab.isActive
    ? setActiveTab([...state.tabs, nextTab], nextTab.id)
    : [...state.tabs, nextTab];
  return changed({
    ...state,
    tabs,
    activeTabId: nextTab.isActive ? nextTab.id : state.activeTabId,
  });
}

export function reduceTabUpdated(
  state: BrowserEventState,
  patch: Partial<FrostTabState> & { tabId: string },
): BrowserEventResult {
  const tabIndex = state.tabs.findIndex((tab) => tab.id === patch.tabId);
  if (tabIndex < 0) return unchanged(state);

  // A windowId in an update is useful as a consistency check, but is not a
  // movable-tab operation. The move event owns that transition.
  if (patch.windowId !== undefined && patch.windowId !== state.windowId) {
    return refresh(state);
  }

  const tabPatch = toTabPatch(patch);
  if (!tabPatch) return unchanged(state);

  const current = state.tabs[tabIndex];
  const nextTab = { ...current, ...tabPatch };
  if (sameTab(current, nextTab)) return unchanged(state);

  const tabs = [...state.tabs];
  tabs[tabIndex] = nextTab;
  return changed({ ...state, tabs });
}

export function reduceTabClosed(
  state: BrowserEventState,
  { tabId }: { tabId: string },
): BrowserEventResult {
  const closedIndex = state.tabs.findIndex((tab) => tab.id === tabId);
  if (closedIndex < 0) return unchanged(state);

  const remaining = state.tabs.filter((tab) => tab.id !== tabId);
  const activeTabId =
    state.activeTabId === tabId
      ? selectAfterTabRemoval(remaining, closedIndex)
      : state.activeTabId;
  const tabs =
    activeTabId && remaining.some((tab) => tab.id === activeTabId)
      ? setActiveTab(remaining, activeTabId)
      : remaining;

  return changed({ ...state, tabs, activeTabId });
}

export function reduceTabActivated(
  state: BrowserEventState,
  { tabId }: { tabId: string },
): BrowserEventResult {
  if (!state.tabs.some((tab) => tab.id === tabId)) return unchanged(state);

  const tabs = setActiveTab(state.tabs, tabId);
  const tabsChanged = tabs.some((tab, index) => tab !== state.tabs[index]);
  if (!tabsChanged && state.activeTabId === tabId) return unchanged(state);

  return changed({ ...state, tabs, activeTabId: tabId });
}

export function reduceTabMoved(
  state: BrowserEventState,
  {
    tabId,
    fromWindowId,
    toWindowId,
    toIndex,
  }: Extract<BrowserEvent, { type: 'tab.moved' }>['payload'],
): BrowserEventResult {
  const currentWindowId = state.windowId;
  const isLocalSource = fromWindowId === currentWindowId;
  const isLocalDestination = toWindowId === currentWindowId;
  if (!isLocalSource && !isLocalDestination) return unchanged(state);

  if (fromWindowId === toWindowId) {
    if (!isLocalSource) return unchanged(state);
    if (!Number.isInteger(toIndex) || toIndex < 0) return refresh(state);

    const currentIndex = state.tabs.findIndex((tab) => tab.id === tabId);
    if (currentIndex < 0) return refresh(state);
    const targetIndex = Math.min(toIndex, state.tabs.length - 1);
    if (currentIndex === targetIndex) return unchanged(state);

    const tabs = reorderTabs(state.tabs, currentIndex, targetIndex);
    return changed({ ...state, tabs });
  }

  if (isLocalSource) {
    const movedIndex = state.tabs.findIndex((tab) => tab.id === tabId);
    if (movedIndex < 0) return refresh(state);

    const remaining = state.tabs.filter((tab) => tab.id !== tabId);
    const activeTabId =
      state.activeTabId === tabId
        ? selectAfterTabRemoval(remaining, movedIndex)
        : state.activeTabId;
    const tabs =
      activeTabId && remaining.some((tab) => tab.id === activeTabId)
        ? setActiveTab(remaining, activeTabId)
        : remaining;
    return changed({ ...state, tabs, activeTabId });
  }

  // The event does not carry the complete tab, so a destination window cannot
  // safely synthesize a tab. Let the snapshot establish the source of truth.
  return refresh(state);
}

export function reduceWindowCreated(
  state: BrowserEventState,
  windowState: FrostWindowState | void,
): BrowserEventResult {
  if (!windowState?.id) return refresh(state);

  const existing = state.windows.find((window) => window.id === windowState.id);
  if (existing) {
    const sameMetadata =
      existing.private === windowState.isPrivate &&
      existing.activeTabId === (windowState.activeTabId ?? '');
    return sameMetadata ? unchanged(state) : refresh(state);
  }

  const nextWindow: WindowSnapshot = {
    id: windowState.id,
    private: windowState.isPrivate,
    activeTabId: windowState.activeTabId ?? '',
    // window.created does not include tab payloads. The next snapshot can
    // enrich this entry without losing any local tab state.
    tabs: [],
  };
  return changed({ ...state, windows: [...state.windows, nextWindow] });
}

export function reduceWindowClosed(
  state: BrowserEventState,
  windowState: { windowId: string } | void,
): BrowserEventResult {
  if (!windowState?.windowId) return refresh(state);
  if (windowState.windowId === state.windowId) return refresh(state);

  const exists = state.windows.some(
    (window) => window.id === windowState.windowId,
  );
  if (!exists) return unchanged(state);

  return changed({
    ...state,
    windows: state.windows.filter(
      (window) => window.id !== windowState.windowId,
    ),
  });
}

export function reduceWindowFocused(
  state: BrowserEventState,
  windowState: { windowId: string } | void,
): BrowserEventResult {
  if (!windowState?.windowId) return refresh(state);
  // A focus event for another native window is broadcast to this bridge too.
  // It has no local state to apply. The current window may need a snapshot
  // because focus can change its active-page context.
  return windowState.windowId === state.windowId
    ? refresh(state)
    : unchanged(state);
}

export function selectAfterTabRemoval(
  remainingTabs: readonly Tab[],
  closedIndex: number,
): string {
  if (remainingTabs.length === 0) return '';
  return remainingTabs[closedIndex === 0 ? 0 : closedIndex - 1]?.id ?? '';
}

export function toTabPatch(
  patch: Partial<FrostTabState> & { tabId: string },
): Partial<Tab> | null {
  const next: Partial<Tab> = {};
  if (patch.title !== undefined) next.title = patch.title;
  if (patch.url !== undefined) next.url = patch.url;
  if (patch.faviconUrl !== undefined) next.faviconUrl = patch.faviconUrl;
  if (patch.errorText !== undefined) next.errorText = patch.errorText;
  if (patch.zoomLevel !== undefined) next.zoomLevel = patch.zoomLevel;
  if (patch.isLoading !== undefined) next.isLoading = patch.isLoading;
  if (patch.canGoBack !== undefined) next.canGoBack = patch.canGoBack;
  if (patch.canGoForward !== undefined) next.canGoForward = patch.canGoForward;
  if (patch.isPinned !== undefined) next.isPinned = patch.isPinned;
  return Object.keys(next).length > 0 ? next : null;
}

function setActiveTab(tabs: readonly Tab[], activeTabId: string): Tab[] {
  return tabs.map((tab) => {
    const isActive = tab.id === activeTabId;
    return tab.isActive === isActive ? tab : { ...tab, isActive };
  });
}

function reorderTabs(
  tabs: readonly Tab[],
  fromIndex: number,
  toIndex: number,
): Tab[] {
  const next = [...tabs];
  const [movedTab] = next.splice(fromIndex, 1);
  next.splice(toIndex, 0, movedTab);
  return next;
}

function sameTab(left: Tab, right: Tab): boolean {
  return (
    left.id === right.id &&
    left.windowId === right.windowId &&
    left.title === right.title &&
    left.url === right.url &&
    left.faviconUrl === right.faviconUrl &&
    left.errorText === right.errorText &&
    left.zoomLevel === right.zoomLevel &&
    left.isLoading === right.isLoading &&
    left.canGoBack === right.canGoBack &&
    left.canGoForward === right.canGoForward &&
    left.isActive === right.isActive &&
    left.isPinned === right.isPinned
  );
}

function fromFrostTab(tab: FrostTabState): Tab {
  return {
    id: tab.id,
    windowId: tab.windowId,
    title: tab.title,
    url: tab.url,
    faviconUrl: tab.faviconUrl,
    errorText: tab.errorText,
    zoomLevel: tab.zoomLevel,
    isLoading: tab.isLoading,
    canGoBack: tab.canGoBack,
    canGoForward: tab.canGoForward,
    isActive: tab.isActive,
    isPinned: tab.isPinned,
  };
}
