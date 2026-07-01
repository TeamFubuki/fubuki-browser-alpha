import { createSignal, For, Show } from "solid-js";
import { fubuki, tabs, type Tab } from "../bridge/fubuki";
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
  const filteredTabs = () => {
    const q = query().trim().toLowerCase();
    if (!q) return browserState.tabs;
    return browserState.tabs.filter((tab) => `${tab.title} ${tab.url}`.toLowerCase().includes(q));
  };

  return (
    <>
      <input class="tab-search" value={query()} placeholder="Search tabs" aria-label="Search tabs" onInput={(event) => setQuery(event.currentTarget.value)} />
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
              <span class="pin-mark" aria-hidden="true">{tab.isPinned ? "●" : ""}</span>
              <Favicon tab={tab} />
              <span class="tab-title">{titleFor(tab)}</span>
            </button>
            <button
              class="tab-action"
              title={tab.isPinned ? "Unpin tab" : "Pin tab"}
              aria-label={tab.isPinned ? "Unpin tab" : "Pin tab"}
              onClick={(event) => {
                event.stopPropagation();
                void tabs.pin(tab.id, !tab.isPinned);
              }}
            >
              <span aria-hidden="true">{tab.isPinned ? "⌾" : "⌽"}</span>
            </button>
            <button
              class="tab-action"
              title="Duplicate tab"
              aria-label={`Duplicate ${titleFor(tab)}`}
              onClick={(event) => {
                event.stopPropagation();
                void tabs.duplicate(tab.id);
              }}
            >
              <span aria-hidden="true">⧉</span>
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
      <div class="tab-bulk-actions">
        <button title="Reopen closed tab" onClick={() => void tabs.reopenClosed()}>↩</button>
        <button title="Close other tabs" disabled={!browserState.activeTabId} onClick={() => void tabs.closeOther(browserState.activeTabId)}>◐</button>
        <button title="Close tabs to the right" disabled={!browserState.activeTabId} onClick={() => void tabs.closeToRight(browserState.activeTabId)}>▸</button>
        <button title="Move tab to new window" disabled={!browserState.activeTabId} onClick={() => void tabs.moveToNewWindow(browserState.activeTabId)}>⇱</button>
      </div>
    </>
  );
}
