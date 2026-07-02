import { createSignal, For, Show } from "solid-js";
import { fubuki, type Tab } from "../bridge/fubuki";
import { browserState } from "../stores/browserStore";

function titleFor(tab: Tab) {
  return tab.title || (tab.url === "fubuki://newtab/" ? "New Tab" : tab.url || "New Tab");
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
  const filteredTabs = () => {
    const q = query().trim().toLowerCase();
    const normalTabs = browserState.tabs.filter((tab) => !tab.isPinned);
    if (!q) return normalTabs;
    return normalTabs.filter((tab) => `${tab.title} ${tab.url}`.toLowerCase().includes(q));
  };
  const pinnedTabs = () => browserState.tabs.filter((tab) => tab.isPinned);
  const showSearch = () => searchExpanded() || browserState.tabs.length >= 8 || query().trim().length > 0;

  return (
    <section class="tab-stack" aria-label="Tabs">
      <Show
        when={showSearch()}
        fallback={
          <button class="tab-search-toggle" title="Search tabs" aria-label="Search tabs" onClick={() => setSearchExpanded(true)}>
            <span aria-hidden="true">⌕</span>
          </button>
        }
      >
        <input
          class="tab-search"
          value={query()}
          placeholder="Search tabs"
          aria-label="Search tabs"
          onInput={(event) => setQuery(event.currentTarget.value)}
          onBlur={() => {
            if (!query().trim()) setSearchExpanded(false);
          }}
        />
      </Show>
      <Show when={pinnedTabs().length > 0}>
        <div class="pinned-tab-list" role="tablist" aria-label="Pinned tabs">
          <For each={pinnedTabs()}>
            {(tab) => (
              <div classList={{ "pinned-tab": true, active: tab.isActive }} title={titleFor(tab)} role="tab" aria-selected={tab.isActive}>
                <button class="pinned-tab-activate" onClick={() => void fubuki.invoke("tabs.activate", { tabId: tab.id })}>
                  <Favicon tab={tab} />
                </button>
              </div>
            )}
          </For>
        </div>
      </Show>
      <div class="vertical-tab-list" role="tablist" aria-label="Open tabs">
        <For each={filteredTabs()}>
          {(tab, index) => (
          <div
            classList={{ "vertical-tab": true, active: tab.isActive, pinned: tab.isPinned }}
            title={titleFor(tab)}
            role="tab"
            aria-selected={tab.isActive}
            draggable
            onDragStart={(event) => event.dataTransfer?.setData("text/plain", tab.id)}
            onDragOver={(event) => event.preventDefault()}
            onDrop={(event) => {
              event.preventDefault();
              const draggedId = event.dataTransfer?.getData("text/plain");
              if (draggedId) void fubuki.invoke("tabs.move", { tabId: draggedId, toIndex: index() });
            }}
          >
            <button class="tab-activate" onClick={() => void fubuki.invoke("tabs.activate", { tabId: tab.id })}>
              <Favicon tab={tab} />
              <span class="tab-title">{titleFor(tab)}</span>
            </button>
            <button
              class="tab-close"
              title="Close tab"
              aria-label={`Close ${titleFor(tab)}`}
              onClick={(event) => {
                event.stopPropagation();
                void fubuki.invoke("tabs.close", { tabId: tab.id });
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
