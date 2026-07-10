import {
  createMemo,
  createSignal,
  For,
  onCleanup,
  onMount,
  Show,
} from 'solid-js';
import { tabs, type Tab } from '../bridge/fubuki';
import { t } from '../i18n';
import {
  browserState,
  currentLanguage,
  setBrowserState,
} from '../stores/browserStore';
import { reorderTabById } from '../tabOrdering';

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
        <img src={props.tab.faviconUrl} alt="" loading="lazy" />
      </Show>
    </span>
  );
}

export default function VerticalTabList() {
  const [query, setQuery] = createSignal('');
  const [searchExpanded, setSearchExpanded] = createSignal(false);
  const [dragOverId, setDragOverId] = createSignal<string | null>(null);
  const [contextMenu, setContextMenu] = createSignal<{
    tabId: string;
    x: number;
    y: number;
  } | null>(null);

  const lang = currentLanguage;

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

  const showSearch = () =>
    searchExpanded() ||
    browserState.tabs.length >= 8 ||
    query().trim().length > 0;

  const handleDrop = async (targetId: string, event: DragEvent) => {
    event.preventDefault();
    setDragOverId(null);
    const dataTransfer = event.dataTransfer;
    if (!dataTransfer) return;
    const draggedId = dataTransfer.getData('text/plain');
    if (!draggedId) return;
    const targetIndex = browserState.tabs.findIndex(
      (item) => item.id === targetId,
    );
    const sourceIndex = browserState.tabs.findIndex(
      (item) => item.id === draggedId,
    );
    if (targetIndex < 0 || sourceIndex < 0 || sourceIndex === targetIndex)
      return;
    try {
      const moved = await tabs.move(draggedId, targetIndex);
      if (!moved) throw new Error('Native rejected tab move');
      setBrowserState(
        'tabs',
        reorderTabById(browserState.tabs, draggedId, targetIndex),
      );
    } catch (error) {
      console.error('[Fubuki] Failed to move tab:', error);
      setBrowserState('status', 'Error');
    }
  };

  onMount(() => {
    const closeMenu = (event: PointerEvent) => {
      const target = event.target;
      if (
        !(target instanceof Element) ||
        !target.closest('.tab-context-menu')
      ) {
        setContextMenu(null);
      }
    };
    const closeMenuOnEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setContextMenu(null);
    };
    window.addEventListener('pointerdown', closeMenu);
    window.addEventListener('keydown', closeMenuOnEscape);
    onCleanup(() => {
      window.removeEventListener('pointerdown', closeMenu);
      window.removeEventListener('keydown', closeMenuOnEscape);
    });
  });

  const openContextMenu = (tabId: string, event: MouseEvent) => {
    event.preventDefault();
    const sidebarRight = (event.currentTarget as Element)
      .closest('.sidebar')
      ?.getBoundingClientRect().right;
    const maxX = (sidebarRight ?? window.innerWidth) - 216;
    setContextMenu({
      tabId,
      x: Math.max(6, Math.min(event.clientX, maxX)),
      y: Math.max(6, Math.min(event.clientY, window.innerHeight - 286)),
    });
  };

  const runTabAction = async (
    tabId: string,
    action: (tabId: string) => Promise<boolean>,
  ) => {
    if (!tabId) return;
    try {
      const ok = await action(tabId);
      if (!ok) throw new Error('Tab action was rejected');
      setContextMenu(null);
    } catch (error) {
      console.error('[Fubuki] Tab action failed:', error);
      setBrowserState('status', 'Error');
    }
  };

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
          aria-label={t('tabs.pinned', lang())}
        >
          <For each={pinnedTabs()}>
            {(tab) => (
              <div
                classList={{ 'pinned-tab': true, active: tab.isActive }}
                title={titleFor(tab, lang())}
                role="tab"
                aria-selected={tab.isActive}
                onContextMenu={(event) => openContextMenu(tab.id, event)}
                onAuxClick={(event) => {
                  if (event.button === 1) void tabs.close(tab.id);
                }}
              >
                <button
                  class="pinned-tab-activate"
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
        aria-label={t('tabs.open', lang())}
      >
        <Show when={browserState.activeTabId}>
          <details class="tab-bulk-menu">
            <summary aria-label={t('tabs.actions', lang())}>•••</summary>
            <div class="tab-bulk-menu-items">
              <button
                onClick={() =>
                  void runTabAction(browserState.activeTabId, tabs.closeOther)
                }
              >
                {t('tabs.closeOther', lang())}
              </button>
              <button
                onClick={() =>
                  void runTabAction(browserState.activeTabId, tabs.closeToRight)
                }
              >
                {t('tabs.closeToRight', lang())}
              </button>
              <button
                onClick={() =>
                  void runTabAction(
                    browserState.activeTabId,
                    tabs.moveToNewWindow,
                  )
                }
              >
                {t('tabs.moveToNewWindow', lang())}
              </button>
            </div>
          </details>
        </Show>
        <For each={filteredTabs()}>
          {(tab) => {
            const closeLabel = `${t('action.closeTab', lang())}: ${titleFor(tab, lang())}`;
            return (
              <div
                classList={{
                  'vertical-tab': true,
                  active: tab.isActive,
                  pinned: tab.isPinned,
                  'drag-over': dragOverId() === tab.id,
                }}
                title={titleFor(tab, lang())}
                role="tab"
                aria-selected={tab.isActive}
                draggable
                onDragStart={(event) => {
                  const dt = event.dataTransfer;
                  if (dt) {
                    dt.setData('text/plain', tab.id);
                    dt.effectAllowed = 'move';
                  }
                }}
                onDragOver={(event) => {
                  event.preventDefault();
                  if (event.dataTransfer)
                    event.dataTransfer.dropEffect = 'move';
                  setDragOverId(tab.id);
                }}
                onDragLeave={() => setDragOverId(null)}
                onDragEnd={() => setDragOverId(null)}
                onDrop={(event) => void handleDrop(tab.id, event)}
                onContextMenu={(event) => openContextMenu(tab.id, event)}
                onAuxClick={(event) => {
                  if (event.button === 1) void tabs.close(tab.id);
                }}
              >
                <button
                  class="tab-activate"
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
                    void tabs.close(tab.id);
                  }}
                >
                  <span aria-hidden="true">x</span>
                </button>
              </div>
            );
          }}
        </For>
      </div>
      <Show when={contextMenu()}>
        {(menu) => {
          const tab = () =>
            browserState.tabs.find((item) => item.id === menu().tabId);
          return (
            <div
              class="tab-context-menu"
              role="menu"
              aria-label={t('tabs.actions', lang())}
              style={{ left: `${menu().x}px`, top: `${menu().y}px` }}
            >
              <button
                role="menuitem"
                onClick={() => void runTabAction(menu().tabId, tabs.reload)}
              >
                {t('common.reload', lang())}
              </button>
              <button
                role="menuitem"
                onClick={() => void runTabAction(menu().tabId, tabs.duplicate)}
              >
                {t('tabs.duplicate', lang())}
              </button>
              <button
                role="menuitem"
                onClick={() =>
                  void runTabAction(menu().tabId, (tabId) =>
                    tabs.pin(tabId, !tab()?.isPinned),
                  )
                }
              >
                {t(tab()?.isPinned ? 'tabs.unpin' : 'tabs.pin', lang())}
              </button>
              <hr />
              <button
                role="menuitem"
                onClick={() => void runTabAction(menu().tabId, tabs.close)}
              >
                {t('action.closeTab', lang())}
              </button>
              <button
                role="menuitem"
                onClick={() => void runTabAction(menu().tabId, tabs.closeOther)}
              >
                {t('tabs.closeOther', lang())}
              </button>
              <button
                role="menuitem"
                onClick={() =>
                  void runTabAction(menu().tabId, tabs.closeToRight)
                }
              >
                {t('tabs.closeToRight', lang())}
              </button>
              <hr />
              <button
                role="menuitem"
                onClick={() =>
                  void runTabAction(menu().tabId, tabs.moveToNewWindow)
                }
              >
                {t('tabs.moveToNewWindow', lang())}
              </button>
            </div>
          );
        }}
      </Show>
    </section>
  );
}
