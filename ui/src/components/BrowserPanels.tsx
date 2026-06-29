import { For, Show, createEffect, createSignal } from "solid-js";
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
  const [searchEngine, setSearchEngine] = createSignal(browserState.settings.searchEngine);
  const [startupBehavior, setStartupBehavior] = createSignal(browserState.settings.startupBehavior);
  const [theme, setTheme] = createSignal(browserState.settings.theme);

  createEffect(() => {
    setHomepage(browserState.settings.homepage);
    setDownloadDirectory(browserState.settings.downloadDirectory);
    setSearchEngine(browserState.settings.searchEngine);
    setStartupBehavior(browserState.settings.startupBehavior);
    setTheme(browserState.settings.theme);
  });

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
              <span>Search engine</span>
              <select value={browserState.settings.searchEngine} onInput={(event) => setSearchEngine(event.currentTarget.value)}>
                <option value="duckduckgo">DuckDuckGo</option>
                <option value="google">Google</option>
                <option value="bing">Bing</option>
              </select>
            </label>
            <label>
              <span>On startup</span>
              <select value={browserState.settings.startupBehavior} onInput={(event) => setStartupBehavior(event.currentTarget.value)}>
                <option value="homepage">Open homepage</option>
                <option value="newTab">Open new tab</option>
              </select>
            </label>
            <label>
              <span>Download folder</span>
              <input value={browserState.settings.downloadDirectory} onInput={(event) => setDownloadDirectory(event.currentTarget.value)} />
            </label>
            <label>
              <span>Theme</span>
              <select value={browserState.settings.theme} onInput={(event) => setTheme(event.currentTarget.value)}>
                <option value="light">Light</option>
                <option value="soft">Soft</option>
                <option value="muted">Muted</option>
                <option value="dark">Dark</option>
              </select>
            </label>
            <button
              onClick={async () => {
                await fubuki.invoke("settings.set", { key: "homepage", value: homepage() });
                await fubuki.invoke("settings.set", { key: "searchEngine", value: searchEngine() });
                await fubuki.invoke("settings.set", { key: "startupBehavior", value: startupBehavior() });
                await fubuki.invoke("settings.set", { key: "downloadDirectory", value: downloadDirectory() });
                await fubuki.invoke("settings.set", { key: "theme", value: theme() });
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
                  <Show when={item.faviconUrl} fallback={<span class="record-favicon" />}>
                    <img class="record-favicon" src={item.faviconUrl} alt="" />
                  </Show>
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
