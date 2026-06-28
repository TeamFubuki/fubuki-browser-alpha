import { createSignal } from "solid-js";
import { fubuki } from "../bridge/fubuki";
import { browserState } from "../stores/browserStore";
import Omnibox from "./Omnibox";

function activeTab() {
  return browserState.tabs.find((tab) => tab.id === browserState.activeTabId);
}

export default function Toolbar() {
  const [draft, setDraft] = createSignal("");

  const submit = () => {
    const tab = activeTab();
    if (tab) {
      void fubuki.invoke("tabs.navigate", { tabId: tab.id, input: draft() || tab.url });
    }
  };

  return (
    <section class="toolbar" aria-label="Navigation">
      <button class="tool-button" title="Back" disabled={!activeTab()?.canGoBack} onClick={() => void fubuki.invoke("tabs.goBack", { tabId: browserState.activeTabId })}>
        ←
      </button>
      <button class="tool-button" title="Forward" disabled={!activeTab()?.canGoForward} onClick={() => void fubuki.invoke("tabs.goForward", { tabId: browserState.activeTabId })}>
        →
      </button>
      <button
        class="tool-button"
        title={activeTab()?.isLoading ? "Stop" : "Reload"}
        onClick={() => void fubuki.invoke(activeTab()?.isLoading ? "tabs.stop" : "tabs.reload", { tabId: browserState.activeTabId })}
      >
        {activeTab()?.isLoading ? "×" : "↻"}
      </button>
      <Omnibox value={activeTab()?.url ?? ""} onDraft={setDraft} onSubmit={submit} />
      <button class="tool-button" title="Bookmark" onClick={() => void fubuki.invoke("bookmarks.addActive")}>
        ☆
      </button>
      <button class="tool-button" title="DevTools" onClick={() => void fubuki.invoke("commands.execute", { id: "app.openDevTools" })}>
        Dev
      </button>
    </section>
  );
}
