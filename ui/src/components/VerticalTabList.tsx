import { createMemo, createSignal, For, onCleanup, Show } from 'solid-js';
import { tabs, type Tab } from '../bridge/fubuki';
import { t } from '../i18n';
import { browserState, currentLanguage } from '../stores/browserStore';
import { tabDropEdge, tabDropIndex, type TabDropTarget } from './tabDrag';
import {
  focusAfterClose,
  focusTabElement,
  focusTargetAfterClose,
  navigationTarget,
  reorderTargetIndex,
  tabIndexFor,
  type TabNavigationKey,
} from './verticalTabKeyboard';

function titleFor(tab: Tab, lang: string) {
  return (
    tab.title ||
    (tab.url === 'fubuki://newtab/'
      ? t('common.newTab', lang)
      : tab.url || t('common.newTab', lang))
  );
}

function Favicon(props: { tab: Tab }) {
  return (
    <span classList={{ 'tab-icon': true, loading: props.tab.isLoading }}>
      <Show when={!props.tab.isLoading && props.tab.faviconUrl}>
        <img
          src={props.tab.faviconUrl}
          alt=""
          loading="lazy"
          draggable={false}
        />
      </Show>
    </span>
  );
}

export default function VerticalTabList() {
  const [query, setQuery] = createSignal('');
  const [searchExpanded, setSearchExpanded] = createSignal(false);
  const [draggedId, setDraggedId] = createSignal<string | null>(null);
  const [dropTarget, setDropTarget] = createSignal<TabDropTarget | null>(null);
  const [focusedTabId, setFocusedTabId] = createSignal('');

  const lang = currentLanguage;
  const focusIds = (list: readonly Tab[]) => list.map((tab) => tab.id);

  const pinnedTabs = createMemo(() =>
    browserState.tabs.filter((tab) => tab.isPinned),
  );
  const normalTabs = createMemo(() =>
    browserState.tabs.filter((tab) => !tab.isPinned),
  );
  const filteredTabs = createMemo(() => {
    const q = query().trim().toLowerCase();
    const list = normalTabs();
    if (!q) return list;
    return list.filter((tab) =>
      `${tab.title} ${tab.url}`.toLowerCase().includes(q),
    );
  });
  const pinnedFocusIds = createMemo(() => focusIds(pinnedTabs()));
  const filteredFocusIds = createMemo(() => focusIds(filteredTabs()));

  const focusAfterCloseAndRestore = (list: readonly Tab[], tab: Tab) => {
    const ids = focusIds(list);
    const nextId = focusAfterClose(ids, ids.indexOf(tab.id));
    setFocusedTabId(nextId ?? '');
    return nextId;
  };

  const closeTab = (list: readonly Tab[], tab: Tab) => {
    const nextId = focusAfterCloseAndRestore(list, tab);
    void tabs.close(tab.id).then((closed) => {
      if (closed) {
        queueMicrotask(() => {
          const focusId = focusTargetAfterClose(
            nextId,
            browserState.activeTabId,
          );
          if (focusId) {
            setFocusedTabId(focusId);
            focusTabElement(focusId);
          }
        });
      }
    });
  };

  const handleTabKeyDown = (
    event: KeyboardEvent,
    list: readonly Tab[],
    tab: Tab,
  ) => {
    const navigationKey = event.key as TabNavigationKey;
    if (
      !event.altKey &&
      (navigationKey === 'ArrowUp' ||
        navigationKey === 'ArrowDown' ||
        navigationKey === 'Home' ||
        navigationKey === 'End')
    ) {
      const targetId = navigationTarget(focusIds(list), tab.id, navigationKey);
      if (targetId) {
        event.preventDefault();
        setFocusedTabId(targetId);
        queueMicrotask(() => focusTabElement(targetId));
      }
      return;
    }

    if (
      event.altKey &&
      (navigationKey === 'ArrowUp' || navigationKey === 'ArrowDown')
    ) {
      const targetIndex = reorderTargetIndex(
        browserState.tabs,
        list,
        tab.id,
        navigationKey,
      );
      if (targetIndex !== null) {
        event.preventDefault();
        void tabs.move(tab.id, targetIndex).then(() => {
          queueMicrotask(() => focusTabElement(tab.id));
        });
      }
      return;
    }

    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      setFocusedTabId(tab.id);
      void tabs.activate(tab.id);
      return;
    }
    if (event.key === 'Delete') {
      event.preventDefault();
      closeTab(list, tab);
    }
  };

  const showSearch = () =>
    searchExpanded() ||
    browserState.tabs.length >= 8 ||
    query().trim().length > 0;

  let pointerDrag: {
    pointerId: number;
    tabId: string;
    startX: number;
    startY: number;
    element: HTMLElement;
  } | null = null;
  let suppressClick = false;

  const clearDrag = () => {
    const drag = pointerDrag;
    pointerDrag = null;
    if (drag?.element.hasPointerCapture(drag.pointerId)) {
      drag.element.releasePointerCapture(drag.pointerId);
    }
    setDraggedId(null);
    setDropTarget(null);
  };
  onCleanup(clearDrag);

  const handlePointerDown = (tab: Tab, event: PointerEvent) => {
    if (event.button !== 0 || !event.isPrimary) return;
    if (!(event.target instanceof HTMLElement)) return;
    if (event.target.closest('.tab-close')) return;
    clearDrag();
    suppressClick = false;
    // Capture the activation button so a normal click still activates the tab.
    const element =
      event.target.closest<HTMLElement>('button') ??
      (event.currentTarget as HTMLElement);
    pointerDrag = {
      pointerId: event.pointerId,
      tabId: tab.id,
      startX: event.clientX,
      startY: event.clientY,
      element,
    };
    element.setPointerCapture(event.pointerId);
  };

  const targetForPointer = (event: PointerEvent): TabDropTarget | null => {
    const row = document
      .elementFromPoint(event.clientX, event.clientY)
      ?.closest<HTMLElement>('[data-drag-tab-id]');
    if (!row || !pointerDrag) return null;
    const tab = browserState.tabs.find(
      (item) => item.id === row.dataset.dragTabId,
    );
    if (!tab) return null;
    const target: TabDropTarget = {
      tabId: tab.id,
      edge: tabDropEdge(event, row.getBoundingClientRect(), tab.isPinned),
    };
    return tabDropIndex(browserState.tabs, pointerDrag.tabId, target) === null
      ? null
      : target;
  };

  const handlePointerMove = (event: PointerEvent) => {
    const drag = pointerDrag;
    if (!drag || drag.pointerId !== event.pointerId) return;
    if (!draggedId()) {
      if (
        Math.hypot(event.clientX - drag.startX, event.clientY - drag.startY) < 5
      )
        return;
      setDraggedId(drag.tabId);
      suppressClick = true;
    }
    event.preventDefault();
    setDropTarget(targetForPointer(event));
  };

  const handlePointerUp = (event: PointerEvent) => {
    const drag = pointerDrag;
    if (!drag || drag.pointerId !== event.pointerId) return;
    const target = draggedId() ? targetForPointer(event) : null;
    const index = target
      ? tabDropIndex(browserState.tabs, drag.tabId, target)
      : null;
    clearDrag();
    if (index !== null) void tabs.move(drag.tabId, index);
  };

  const activateTab = (tab: Tab, event: MouseEvent) => {
    if (suppressClick && event.detail > 0) {
      event.preventDefault();
      return;
    }
    void tabs.activate(tab.id);
  };

  const dropClass = (tabId: string, edge: TabDropTarget['edge']) =>
    dropTarget()?.tabId === tabId && dropTarget()?.edge === edge;

  return (
    <section class="tab-stack" aria-label={t('common.tabs', lang())}>
      <Show
        when={showSearch()}
        fallback={
          <button
            class="tab-search-toggle"
            title={t('tabs.search', lang())}
            aria-label={t('tabs.search', lang())}
            onClick={() => setSearchExpanded(true)}
          >
            <span aria-hidden="true">⌕</span>
          </button>
        }
      >
        <input
          class="tab-search"
          value={query()}
          placeholder={t('tabs.search', lang())}
          aria-label={t('tabs.search', lang())}
          onInput={(event) => setQuery(event.currentTarget.value)}
          onBlur={() => {
            if (!query().trim()) setSearchExpanded(false);
          }}
        />
      </Show>
      <Show when={pinnedTabs().length > 0}>
        <div
          class="pinned-tab-list"
          role="tablist"
          aria-orientation="vertical"
          aria-label={t('tabs.pinned', lang())}
        >
          <For each={pinnedTabs()}>
            {(tab) => (
              <div
                classList={{
                  'pinned-tab': true,
                  active: tab.isActive,
                  dragging: draggedId() === tab.id,
                  'drop-before': dropClass(tab.id, 'before'),
                  'drop-after': dropClass(tab.id, 'after'),
                }}
                title={titleFor(tab, lang())}
                role="presentation"
                data-drag-tab-id={tab.id}
                onPointerDown={(event) => handlePointerDown(tab, event)}
                onPointerMove={handlePointerMove}
                onPointerUp={handlePointerUp}
                onPointerCancel={clearDrag}
                onLostPointerCapture={clearDrag}
              >
                <button
                  class="pinned-tab-activate"
                  data-tab-id={tab.id}
                  role="tab"
                  aria-label={titleFor(tab, lang())}
                  aria-selected={tab.isActive}
                  aria-keyshortcuts="ArrowUp ArrowDown Home End Enter Space Delete Alt+ArrowUp Alt+ArrowDown"
                  tabIndex={tabIndexFor(
                    tab.id,
                    pinnedFocusIds(),
                    browserState.activeTabId,
                    focusedTabId(),
                  )}
                  onFocus={() => setFocusedTabId(tab.id)}
                  onKeyDown={(event) =>
                    handleTabKeyDown(event, pinnedTabs(), tab)
                  }
                  onClick={(event) => activateTab(tab, event)}
                >
                  <Favicon tab={tab} />
                </button>
              </div>
            )}
          </For>
        </div>
      </Show>
      <div
        class="vertical-tab-list"
        role="tablist"
        aria-orientation="vertical"
        aria-label={t('tabs.open', lang())}
      >
        <For each={filteredTabs()}>
          {(tab) => {
            const closeLabel = `${t('action.closeTab', lang())}: ${titleFor(tab, lang())}`;
            return (
              <div
                classList={{
                  'vertical-tab': true,
                  active: tab.isActive,
                  pinned: tab.isPinned,
                  dragging: draggedId() === tab.id,
                  'drop-before': dropClass(tab.id, 'before'),
                  'drop-after': dropClass(tab.id, 'after'),
                }}
                role="presentation"
                title={titleFor(tab, lang())}
                data-drag-tab-id={tab.id}
                onPointerDown={(event) => handlePointerDown(tab, event)}
                onPointerMove={handlePointerMove}
                onPointerUp={handlePointerUp}
                onPointerCancel={clearDrag}
                onLostPointerCapture={clearDrag}
              >
                <button
                  class="tab-activate"
                  data-tab-id={tab.id}
                  role="tab"
                  tabIndex={tabIndexFor(
                    tab.id,
                    filteredFocusIds(),
                    browserState.activeTabId,
                    focusedTabId(),
                  )}
                  aria-label={titleFor(tab, lang())}
                  aria-selected={tab.isActive}
                  aria-keyshortcuts="ArrowUp ArrowDown Home End Enter Space Delete Alt+ArrowUp Alt+ArrowDown"
                  onFocus={() => setFocusedTabId(tab.id)}
                  onKeyDown={(event) =>
                    handleTabKeyDown(event, filteredTabs(), tab)
                  }
                  onClick={(event) => activateTab(tab, event)}
                >
                  <Favicon tab={tab} />
                  <span class="tab-title">{titleFor(tab, lang())}</span>
                </button>
                <button
                  class="tab-close"
                  title={t('action.closeTab', lang())}
                  aria-label={closeLabel}
                  onClick={(event) => {
                    event.stopPropagation();
                    closeTab(filteredTabs(), tab);
                  }}
                >
                  <span aria-hidden="true">x</span>
                </button>
              </div>
            );
          }}
        </For>
      </div>
    </section>
  );
}
