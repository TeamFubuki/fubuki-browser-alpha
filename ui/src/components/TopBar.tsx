import { fubuki } from "../bridge/fubuki";
import { browserState, refreshState } from "../stores/browserStore";
import Omnibox from "./Omnibox";

function activeTab() {
  return browserState.tabs.find((tab) => tab.id === browserState.activeTabId);
}

function isBookmarked() {
  const tab = activeTab();
  return !!tab?.url && browserState.bookmarks.some((bookmark) => bookmark.url === tab.url);
}

async function toggleBookmark() {
  const tab = activeTab();
  if (!tab?.url || tab.url.startsWith("fubuki://") || tab.url.startsWith("data:")) return;
  if (isBookmarked()) {
    await fubuki.invoke("bookmarks.remove", { url: tab.url });
  } else {
    await fubuki.invoke("bookmarks.save", {
      title: tab.title || tab.url,
      url: tab.url,
      faviconUrl: tab.faviconUrl || ""
    });
  }
  await refreshState("bookmarks.changed");
}

export default function TopBar() {
  return (
    <header class="top-bar" aria-label="Navigation">
      <button class="topbar-button" title="Back" aria-label="Back" disabled={!activeTab()?.canGoBack} onClick={() => void fubuki.invoke("tabs.goBack", { tabId: browserState.activeTabId })}>
        <span aria-hidden="true">←</span>
      </button>
      <button class="topbar-button" title="Forward" aria-label="Forward" disabled={!activeTab()?.canGoForward} onClick={() => void fubuki.invoke("tabs.goForward", { tabId: browserState.activeTabId })}>
        <span aria-hidden="true">→</span>
      </button>
      <button
        class="topbar-button"
        title={activeTab()?.isLoading ? "Stop" : "Reload"}
        aria-label={activeTab()?.isLoading ? "Stop" : "Reload"}
        disabled={!activeTab()}
        onClick={() => void fubuki.invoke(activeTab()?.isLoading ? "tabs.stop" : "tabs.reload", { tabId: browserState.activeTabId })}
      >
        <span aria-hidden="true">{activeTab()?.isLoading ? "×" : "↻"}</span>
      </button>
      <Omnibox />
      <button
        classList={{ "topbar-button": true, bookmarked: isBookmarked() }}
        title={isBookmarked() ? "Remove bookmark" : "Add bookmark"}
        aria-label={isBookmarked() ? "Remove bookmark" : "Add bookmark"}
        disabled={!activeTab()?.url || activeTab()?.url.startsWith("fubuki://")}
        onClick={() => void toggleBookmark()}
      >
        <span aria-hidden="true">{isBookmarked() ? "★" : "☆"}</span>
      </button>
    </header>
  );
}
