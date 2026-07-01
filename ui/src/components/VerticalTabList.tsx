import { For, Show } from "solid-js";
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
  return (
    <div class="vertical-tab-list" role="tablist" aria-label="Open tabs">
      <For each={browserState.tabs}>
        {(tab) => (
          <div classList={{ "vertical-tab": true, active: tab.isActive }} title={titleFor(tab)} role="tab" aria-selected={tab.isActive}>
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
  );
}
