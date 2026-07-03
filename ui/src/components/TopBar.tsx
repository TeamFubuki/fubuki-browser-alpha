import { createSignal, onCleanup, onMount } from "solid-js";
import { invokeBridge, page } from "../bridge/fubuki";
import { t } from "../i18n";
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
    await invokeBridge("bookmarks.remove", { url: tab.url });
  } else {
    await invokeBridge("bookmarks.save", {
      title: tab.title || tab.url,
      url: tab.url,
      faviconUrl: tab.faviconUrl || ""
    });
  }
  await refreshState("bookmarks.changed");
}

export default function TopBar() {
  const [findOpen, setFindOpen] = createSignal(false);
  const [findText, setFindText] = createSignal("");

  onMount(() => {
    const showFind = () => setFindOpen(true);
    window.addEventListener("fubuki:show-find", showFind);
    onCleanup(() => window.removeEventListener("fubuki:show-find", showFind));
  });

  const submitFind = (forward = true) => {
    const query = findText().trim();
    if (query) void page.find(query, forward);
  };

  return (
    <header classList={{ "top-bar": true, private: browserState.isPrivate }} aria-label="Navigation">
      <button class="topbar-button" title={t("common.back", browserState.settings.language)} aria-label={t("common.back", browserState.settings.language)} disabled={!activeTab()?.canGoBack} onClick={() => void invokeBridge("tabs.goBack", { tabId: browserState.activeTabId })}>
        <span aria-hidden="true">←</span>
      </button>
      <button class="topbar-button" title={t("common.forward", browserState.settings.language)} aria-label={t("common.forward", browserState.settings.language)} disabled={!activeTab()?.canGoForward} onClick={() => void invokeBridge("tabs.goForward", { tabId: browserState.activeTabId })}>
        <span aria-hidden="true">→</span>
      </button>
      <button
        class="topbar-button"
        title={activeTab()?.isLoading ? t("common.stop", browserState.settings.language) : t("common.reload", browserState.settings.language)}
        aria-label={activeTab()?.isLoading ? t("common.stop", browserState.settings.language) : t("common.reload", browserState.settings.language)}
        disabled={!activeTab()}
        onClick={() => void invokeBridge(activeTab()?.isLoading ? "tabs.stop" : "tabs.reload", { tabId: browserState.activeTabId })}
      >
        <span aria-hidden="true">{activeTab()?.isLoading ? "×" : "↻"}</span>
      </button>
      <Omnibox />
      <button
        classList={{ "topbar-button": true, bookmarked: isBookmarked() }}
        title={isBookmarked() ? t("action.removeBookmark", browserState.settings.language) : t("action.addBookmark", browserState.settings.language)}
        aria-label={isBookmarked() ? t("action.removeBookmark", browserState.settings.language) : t("action.addBookmark", browserState.settings.language)}
        disabled={!activeTab()?.url || activeTab()?.url.startsWith("fubuki://") || activeTab()?.url.startsWith("data:")}
        onClick={() => void toggleBookmark()}
      >
        <span aria-hidden="true">{isBookmarked() ? "★" : "☆"}</span>
      </button>
      {findOpen() && (
        <form
          class="find-bar"
          onSubmit={(event) => {
            event.preventDefault();
            submitFind(true);
          }}
        >
          <input value={findText()} placeholder={t("common.find", browserState.settings.language)} aria-label={t("common.find", browserState.settings.language)} onInput={(event) => setFindText(event.currentTarget.value)} autofocus />
          <button type="button" title={t("find.previous", browserState.settings.language)} onClick={() => submitFind(false)}>↑</button>
          <button type="button" title={t("find.next", browserState.settings.language)} onClick={() => submitFind(true)}>↓</button>
          <button
            type="button"
            title={t("action.closeFind", browserState.settings.language)}
            onClick={() => {
              setFindOpen(false);
              void page.stopFinding();
            }}
          >
            ×
          </button>
        </form>
      )}
    </header>
  );
}
