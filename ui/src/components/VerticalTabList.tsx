import { createSignal, For, Show } from "solid-js";
import { invokeBridge, tabs, type Tab } from "../bridge/fubuki";
import { t } from "../i18n";
import { browserState, currentLanguage } from "../stores/browserStore";

function titleFor(tab: Tab, lang: string) {
  return (
    tab.title ||
    (tab.url === "fubuki://newtab/"
      ? t("common.newTab", lang)
      : tab.url || t("common.newTab", lang))
  );
}

function Favicon(props: { tab: Tab }) {
  return (
    <span classList={{ "tab-icon": true, loading: props.tab.isLoading }}>
      <Show when={!props.tab.isLoading && props.tab.faviconUrl}>
        <img src={props.tab.faviconUrl} alt="" />
      </Show>
    </span>
  );
}

export default function VerticalTabList() {
  const [query, setQuery] = createSignal("");
  const [searchExpanded, setSearchExpanded] = createSignal(false);
  const [dragOverId, setDragOverId] = createSignal<string | null>(null);

  const filteredTabs = () => {
    const q = query().trim().toLowerCase();
    const normalTabs = browserState.tabs.filter((tab) => !tab.isPinned);
    if (!q) return normalTabs;
    return normalTabs.filter((tab) =>
      `${tab.title} ${tab.url}`.toLowerCase().includes(q)
    );
  };
  const pinnedTabs = () => browserState.tabs.filter((tab) => tab.isPinned);
  const showSearch = () =>
    searchExpanded() ||
    browserState.tabs.length >= 8 ||
    query().trim().length > 0;

  return (
    <section class="tab-stack" aria-label={t("common.tabs", currentLanguage())}>
      <Show
        when={showSearch()}
        fallback={
          <button
            class="tab-search-toggle"
            title={t("tabs.search", currentLanguage())}
            aria-label={t("tabs.search", currentLanguage())}
            onClick={() => setSearchExpanded(true)}
          >
            <span aria-hidden="true">⌕</span>
          </button>
        }
      >
        <input
          class="tab-search"
          value={query()}
          placeholder={t("tabs.search", currentLanguage())}
          aria-label={t("tabs.search", currentLanguage())}
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
          aria-label={t("tabs.pinned", currentLanguage())}
        >
          <For each={pinnedTabs()}>
            {(tab) => (
              <div
                classList={{ "pinned-tab": true, active: tab.isActive }}
                title={titleFor(tab, currentLanguage())}
                role="tab"
                aria-selected={tab.isActive}
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
        aria-label={t("tabs.open", currentLanguage())}
      >
        <For each={filteredTabs()}>
          {(tab) => (
            <div
              classList={{
                "vertical-tab": true,
                active: tab.isActive,
                pinned: tab.isPinned,
                "drag-over": dragOverId() === tab.id,
              }}
              title={titleFor(tab, currentLanguage())}
              role="tab"
              aria-selected={tab.isActive}
              draggable
              onDragStart={(event) => {
                event.dataTransfer?.setData("text/plain", tab.id);
                event.dataTransfer!.effectAllowed = "move";
              }}
              onDragOver={(event) => {
                event.preventDefault();
                event.dataTransfer!.dropEffect = "move";
                setDragOverId(tab.id);
              }}
              onDragLeave={() => {
                setDragOverId(null);
              }}
              onDrop={(event) => {
                event.preventDefault();
                setDragOverId(null);
                const draggedId = event.dataTransfer?.getData("text/plain");
                const targetIndex = browserState.tabs.findIndex(
                  (item) => item.id === tab.id
                );
                if (draggedId && targetIndex >= 0)
                  void invokeBridge("tabs.move", {
                    tabId: draggedId,
                    toIndex: targetIndex,
                  });
              }}
            >
              <button
                class="tab-activate"
                onClick={() => void tabs.activate(tab.id)}
              >
                <Favicon tab={tab} />
                <span class="tab-title">{titleFor(tab, currentLanguage())}</span>
              </button>
              <button
                class="tab-close"
                title={t("action.closeTab", currentLanguage())}
                aria-label={`${t("action.closeTab", currentLanguage())}: ${titleFor(tab, currentLanguage())}`}
                onClick={(event) => {
                  event.stopPropagation();
                  void tabs.close(tab.id);
                }}
              >
                <span aria-hidden="true">x</span>
              </button>
            </div>
          )}
        </For>
      </div>
    </section>
  );
}
