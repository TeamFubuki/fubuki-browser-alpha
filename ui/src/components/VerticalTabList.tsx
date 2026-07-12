import { createMemo, createSignal, For, Show } from 'solid-js';
import { invokeBridge, tabs, type Tab } from '../bridge/fubuki';
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

  const showContextMenu = (tabId: string, x: number, y: number) => {
    void invokeBridge('ui.showTabContextMenu', { tabId, x, y })
      .then((shown) => {
        if (!shown) throw new Error('Native rejected the tab menu request');
      })
      .catch((error) => {
        console.error('[Fubuki] Failed to show the native tab menu:', error);
      });
  };

  const openContextMenu = (tabId: string, event: MouseEvent) => {
    event.preventDefault();
    showContextMenu(tabId, event.clientX, event.clientY);
  };

  const openContextMenuFromKeyboard = (tabId: string, event: KeyboardEvent) => {
    if (event.key !== 'ContextMenu' && !(event.shiftKey && event.key === 'F10'))
      return;
    event.preventDefault();
    const target = event.currentTarget;
    if (!(target instanceof HTMLElement)) return;
    const bounds = target.getBoundingClientRect();
    showContextMenu(tabId, bounds.left + 24, bounds.bottom);
  };

  const openContextMenuFromButton = (tabId: string, event: MouseEvent) => {
    event.stopPropagation();
    const target = event.currentTarget;
    if (!(target instanceof HTMLElement)) return;
    const bounds = target.getBoundingClientRect();
    showContextMenu(tabId, bounds.right, bounds.bottom);
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
                aria-haspopup="menu"
                tabIndex={0}
                onContextMenu={(event) => openContextMenu(tab.id, event)}
                onKeyDown={(event) =>
                  openContextMenuFromKeyboard(tab.id, event)
                }
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
                <button
                  class="tab-menu"
                  type="button"
                  title={t('tabs.menu', lang())}
                  aria-label={`${t('tabs.menu', lang())}: ${titleFor(tab, lang())}`}
                  aria-haspopup="menu"
                  onClick={(event) => openContextMenuFromButton(tab.id, event)}
                >
                  <span aria-hidden="true">…</span>
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
                aria-haspopup="menu"
                tabIndex={0}
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
                onKeyDown={(event) =>
                  openContextMenuFromKeyboard(tab.id, event)
                }
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
                <button
                  class="tab-menu"
                  type="button"
                  title={t('tabs.menu', lang())}
                  aria-label={`${t('tabs.menu', lang())}: ${titleFor(tab, lang())}`}
                  aria-haspopup="menu"
                  onClick={(event) => openContextMenuFromButton(tab.id, event)}
                >
                  <span aria-hidden="true">…</span>
                </button>
              </div>
            );
          }}
        </For>
      </div>
    </section>
  );
}
