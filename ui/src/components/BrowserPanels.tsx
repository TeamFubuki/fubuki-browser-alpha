import { For, Show, createSignal } from "solid-js";
import { fubuki, type BrowserRecord } from "../bridge/fubuki";
import { browserState, refreshState } from "../stores/browserStore";

type Panel = "bookmarks" | "history" | "downloads" | "settings" | "logs";

function openRecord(item: BrowserRecord) {
  if (item.url && browserState.activeTabId) {
    void fubuki.invoke("tabs.navigate", { tabId: browserState.activeTabId, input: item.url });
  }
}

export default function BrowserPanels() {
  const [panel, setPanel] = createSignal<Panel>("bookmarks");
  const [homepage, setHomepage] = createSignal(browserState.settings.homepage);
  const [downloadDirectory, setDownloadDirectory] = createSignal(browserState.settings.downloadDirectory);

  const records = () => {
    if (panel() === "bookmarks") return browserState.bookmarks;
    if (panel() === "history") return browserState.history;
    if (panel() === "downloads") return browserState.downloads;
    if (panel() === "logs") return browserState.logs;
    return [];
  };

  return (
    <section class="browser-panel">
      <nav class="panel-tabs">
        <button classList={{ selected: panel() === "bookmarks" }} onClick={() => setPanel("bookmarks")}>Bookmarks</button>
        <button classList={{ selected: panel() === "history" }} onClick={() => setPanel("history")}>History</button>
        <button classList={{ selected: panel() === "downloads" }} onClick={() => setPanel("downloads")}>Downloads</button>
        <button classList={{ selected: panel() === "settings" }} onClick={() => setPanel("settings")}>Settings</button>
        <button classList={{ selected: panel() === "logs" }} onClick={() => setPanel("logs")}>Logs</button>
      </nav>

      <Show
        when={panel() !== "settings"}
        fallback={
          <div class="settings-grid">
            <label>
              <span>Homepage</span>
              <input value={browserState.settings.homepage} onInput={(event) => setHomepage(event.currentTarget.value)} />
            </label>
            <label>
              <span>Download folder</span>
              <input value={browserState.settings.downloadDirectory} onInput={(event) => setDownloadDirectory(event.currentTarget.value)} />
            </label>
            <button
              onClick={async () => {
                await fubuki.invoke("settings.set", { key: "homepage", value: homepage() });
                await fubuki.invoke("settings.set", { key: "downloadDirectory", value: downloadDirectory() });
                await refreshState("settings.saved");
              }}
            >
              Save
            </button>
            <p class="profile-path">{browserState.profilePath}</p>
          </div>
        }
      >
        <div class="record-list">
          <For each={records()}>
            {(item) => (
              <div class="record-row">
                <button onClick={() => openRecord(item)}>
                  <span>{item.title || item.path || item.message || item.url || "Untitled"}</span>
                  <small>{item.url || item.path || item.createdAt}</small>
                </button>
                <Show when={panel() === "bookmarks"}>
                  <button class="small-action" onClick={() => void fubuki.invoke("bookmarks.remove", { url: item.url }).then(() => refreshState("bookmark.removed"))}>Remove</button>
                </Show>
                <Show when={panel() === "downloads"}>
                  <small class="download-state">{item.state} {item.percent ?? 0}%</small>
                </Show>
              </div>
            )}
          </For>
        </div>
      </Show>
    </section>
  );
}
