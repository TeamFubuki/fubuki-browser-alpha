import { createSignal } from "solid-js";
import { fubuki } from "../bridge/fubuki";
import { browserState, refreshState } from "../stores/browserStore";
import BookmarkModal from "./BookmarkModal";
import DownloadsModal from "./DownloadsModal";
import Omnibox from "./Omnibox";

function activeTab() {
  return browserState.tabs.find((tab) => tab.id === browserState.activeTabId);
}

export default function Toolbar() {
  const [draft, setDraft] = createSignal("");
  const [bookmarksOpen, setBookmarksOpen] = createSignal(false);
  const [downloadsOpen, setDownloadsOpen] = createSignal(false);

  const submit = () => {
    const tab = activeTab();
    if (tab) {
      void fubuki.invoke("tabs.navigate", { tabId: tab.id, input: draft() || tab.url });
    }
  };

  const isBookmarked = () => {
    const tab = activeTab();
    return !!tab?.url && browserState.bookmarks.some((bookmark) => bookmark.url === tab.url);
  };

  const toggleBookmark = async () => {
    const tab = activeTab();
    if (!tab?.url) return;
    if (isBookmarked()) {
      await fubuki.invoke("bookmarks.remove", { url: tab.url });
    } else {
      await fubuki.invoke("bookmarks.addActive");
    }
    await refreshState("bookmarks.changed");
  };

  const label = (en: string, ja: string) => (browserState.settings.language === "ja" ? ja : en);

  return (
    <section class="toolbar" aria-label="Navigation">
      <button class="tool-button" title={label("Back", "戻る")} disabled={!activeTab()?.canGoBack} onClick={() => void fubuki.invoke("tabs.goBack", { tabId: browserState.activeTabId })}>
        <span aria-hidden="true">←</span>
      </button>
      <button class="tool-button" title={label("Forward", "進む")} disabled={!activeTab()?.canGoForward} onClick={() => void fubuki.invoke("tabs.goForward", { tabId: browserState.activeTabId })}>
        <span aria-hidden="true">→</span>
      </button>
      <button
        class="tool-button"
        title={activeTab()?.isLoading ? label("Stop", "停止") : label("Reload", "再読み込み")}
        onClick={() => void fubuki.invoke(activeTab()?.isLoading ? "tabs.stop" : "tabs.reload", { tabId: browserState.activeTabId })}
      >
        <span aria-hidden="true">{activeTab()?.isLoading ? "×" : "↻"}</span>
      </button>
      <Omnibox value={activeTab()?.url ?? ""} onDraft={setDraft} onSubmit={submit} />
      <button classList={{ "tool-button": true, selected: isBookmarked() }} title={isBookmarked() ? label("Remove bookmark", "ブックマークを削除") : label("Add bookmark", "ブックマークを追加")} onClick={() => void toggleBookmark()}>
        <span aria-hidden="true">{isBookmarked() ? "★" : "☆"}</span>
      </button>
      <button class="tool-button" title={label("Bookmarks", "ブックマーク")} onClick={() => setBookmarksOpen(true)}>
        <span aria-hidden="true">▤</span>
      </button>
      <button class="tool-button" title={label("Downloads", "ダウンロード")} onClick={() => setDownloadsOpen(true)}>
        <span aria-hidden="true">⇩</span>
      </button>
      <button class="tool-button" title={label("Settings", "設定")} onClick={() => browserState.activeTabId && void fubuki.invoke("tabs.navigate", { tabId: browserState.activeTabId, input: "fubuki://settings/" })}>
        <span aria-hidden="true">⚙</span>
      </button>
      <BookmarkModal open={bookmarksOpen()} onClose={() => setBookmarksOpen(false)} />
      <DownloadsModal open={downloadsOpen()} onClose={() => setDownloadsOpen(false)} />
    </section>
  );
}
