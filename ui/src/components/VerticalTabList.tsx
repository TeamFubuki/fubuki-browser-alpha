import { createMemo, createSignal, For, Show } from 'solid-js';
import { tabs, type Tab } from '../bridge/fubuki';
import { t } from '../i18n';
import { browserState, currentLanguage } from '../stores/browserStore';
import {
  TAB_DRAG_TYPE,
  tabDropEdge,
  tabDropIndex,
  type TabDropTarget,
} from './tabDrag';
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

  const clearDrag = () => {
    setDraggedId(null);
    setDropTarget(null);
  };

  const handleDragStart = (tab: Tab, event: DragEvent) => {
    if (!event.dataTransfer) {
      event.preventDefault();
      return;
    }
    event.dataTransfer.setData(TAB_DRAG_TYPE, tab.id);
    event.dataTransfer.effectAllowed = 'move';
    setDraggedId(tab.id);
    setDropTarget(null);
  };

  const targetFor = (tab: Tab, event: DragEvent, element: HTMLElement) => {
    const sourceId = draggedId();
    if (!sourceId || !event.dataTransfer?.types.includes(TAB_DRAG_TYPE))
      return null;
    const target: TabDropTarget = {
      tabId: tab.id,
      edge: tabDropEdge(event, element.getBoundingClientRect(), tab.isPinned),
    };
    return tabDropIndex(browserState.tabs, sourceId, target) === null
      ? null
      : target;
  };

  const handleDragOver = (tab: Tab, event: DragEvent, element: HTMLElement) => {
    const target = targetFor(tab, event, element);
    setDropTarget(target);
    if (!target) return;
    event.preventDefault();
    if (event.dataTransfer) event.dataTransfer.dropEffect = 'move';
  };

  const handleDragLeave = (event: DragEvent, element: HTMLElement) => {
    if (
      event.relatedTarget instanceof Node &&
      element.contains(event.relatedTarget)
    )
      return;
    setDropTarget(null);
  };

  const handleDrop = (tab: Tab, event: DragEvent, element: HTMLElement) => {
    const sourceId = draggedId();
    const target = targetFor(tab, event, element);
    const payloadId = event.dataTransfer?.getData(TAB_DRAG_TYPE);
    clearDrag();
    if (!sourceId || payloadId !== sourceId || !target) return;
    event.preventDefault();
    const index = tabDropIndex(browserState.tabs, sourceId, target);
    if (index !== null) void tabs.move(sourceId, index);
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
                draggable
                onDragStart={(event) => handleDragStart(tab, event)}
                onDragOver={(event) =>
                  handleDragOver(tab, event, event.currentTarget)
                }
                onDragLeave={(event) =>
                  handleDragLeave(event, event.currentTarget)
                }
                onDragEnd={clearDrag}
                onDrop={(event) => handleDrop(tab, event, event.currentTarget)}
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
                  onClick={() => void tabs.activate(tab.id)}
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
                draggable
                onDragStart={(event) => handleDragStart(tab, event)}
                onDragOver={(event) =>
                  handleDragOver(tab, event, event.currentTarget)
                }
                onDragLeave={(event) =>
                  handleDragLeave(event, event.currentTarget)
                }
                onDragEnd={clearDrag}
                onDrop={(event) => handleDrop(tab, event, event.currentTarget)}
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
                  onClick={() => void tabs.activate(tab.id)}
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
