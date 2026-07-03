import { createSignal, onCleanup, onMount } from "solid-js";
import { invokeBridge, page } from "../bridge/fubuki";
import { t } from "../i18n";
import {
  activeTab,
  browserState,
  isTabBookmarked,
  refreshState,
} from "../stores/browserStore";
import Omnibox from "./Omnibox";

function toggleBookmark() {
  const tab = activeTab();
  if (!tab?.url || tab.url.startsWith("fubuki://") || tab.url.startsWith("data:"))
    return;
  if (isTabBookmarked(tab.url)) {
    void invokeBridge("bookmarks.remove", { url: tab.url });
  } else {
    void invokeBridge("bookmarks.save", {
      title: tab.title || tab.url,
      url: tab.url,
      faviconUrl: tab.faviconUrl || "",
    });
  }
  void refreshState("bookmarks.changed");
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

  const tab = activeTab;
  const lang = browserState.settings.language;

  return (
    <header classList={{ "top-bar": true, private: browserState.isPrivate }} aria-label="Navigation">
      <button class="topbar-button" title={t("common.back", lang)} aria-label={t("common.back", lang)} disabled={!tab()?.canGoBack} onClick={() => void invokeBridge("tabs.goBack", { tabId: browserState.activeTabId })}>
        <span aria-hidden="true">←</span>
      </button>
      <button class="topbar-button" title={t("common.forward", lang)} aria-label={t("common.forward", lang)} disabled={!tab()?.canGoForward} onClick={() => void invokeBridge("tabs.goForward", { tabId: browserState.activeTabId })}>
        <span aria-hidden="true">→</span>
      </button>
      <button
        class="topbar-button"
        title={tab()?.isLoading ? t("common.stop", lang) : t("common.reload", lang)}
        aria-label={tab()?.isLoading ? t("common.stop", lang) : t("common.reload", lang)}
        disabled={!tab()}
        onClick={() => void invokeBridge(tab()?.isLoading ? "tabs.stop" : "tabs.reload", { tabId: browserState.activeTabId })}
      >
        <span aria-hidden="true">{tab()?.isLoading ? "×" : "↻"}</span>
      </button>
      <Omnibox />
      <button
        classList={{ "topbar-button": true, bookmarked: isTabBookmarked(tab()?.url) }}
        title={isTabBookmarked(tab()?.url) ? t("action.removeBookmark", lang) : t("action.addBookmark", lang)}
        aria-label={isTabBookmarked(tab()?.url) ? t("action.removeBookmark", lang) : t("action.addBookmark", lang)}
        disabled={!tab()?.url || tab()?.url.startsWith("fubuki://") || tab()?.url.startsWith("data:")}
        onClick={() => void toggleBookmark()}
      >
        <span aria-hidden="true">{isTabBookmarked(tab()?.url) ? "★" : "☆"}</span>
      </button>
      {findOpen() && (
        <form
          class="find-bar"
          onSubmit={(event) => {
            event.preventDefault();
            submitFind(true);
          }}
        >
          <input value={findText()} placeholder={t("common.find", lang)} aria-label={t("common.find", lang)} onInput={(event) => setFindText(event.currentTarget.value)} autofocus />
          <button type="button" title={t("find.previous", lang)} onClick={() => submitFind(false)}>↑</button>
          <button type="button" title={t("find.next", lang)} onClick={() => submitFind(true)}>↓</button>
          <button
            type="button"
            title={t("action.closeFind", lang)}
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
