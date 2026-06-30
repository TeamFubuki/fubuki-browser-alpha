import { Show, createEffect, createSignal, onCleanup } from "solid-js";
import { fubuki } from "../bridge/fubuki";
import { browserState, refreshState } from "../stores/browserStore";
import Omnibox from "./Omnibox";

type Props = {
  onBookmarkEdit: (anchor?: HTMLElement) => void;
  onBookmarks: (anchor?: HTMLElement) => void;
  onHistory: (anchor?: HTMLElement) => void;
  onDownloads: (anchor?: HTMLElement) => void;
  onSettings: () => void;
  onOverlayActive: (active: boolean) => void;
};

function activeTab() {
  return browserState.tabs.find((tab) => tab.id === browserState.activeTabId);
}

function label(en: string, ja: string) {
  return browserState.settings.language === "ja" ? ja : en;
}

function navigate(url: string) {
  if (browserState.activeTabId) {
    void fubuki.invoke("tabs.navigate", { tabId: browserState.activeTabId, input: url });
  } else {
    void fubuki.invoke("tabs.create", { url, active: true });
  }
}

function NavigationControls() {
  return (
    <div class="navigation-controls">
      <button class="tool-button" title={label("Back", "戻る")} aria-label={label("Back", "戻る")} disabled={!activeTab()?.canGoBack} onClick={() => void fubuki.invoke("tabs.goBack", { tabId: browserState.activeTabId })}>
        <span aria-hidden="true">←</span>
      </button>
      <button class="tool-button" title={label("Forward", "進む")} aria-label={label("Forward", "進む")} disabled={!activeTab()?.canGoForward} onClick={() => void fubuki.invoke("tabs.goForward", { tabId: browserState.activeTabId })}>
        <span aria-hidden="true">→</span>
      </button>
      <button
        class="tool-button"
        title={activeTab()?.isLoading ? label("Stop", "停止") : label("Reload", "再読み込み")}
        aria-label={activeTab()?.isLoading ? label("Stop", "停止") : label("Reload", "再読み込み")}
        onClick={() => void fubuki.invoke(activeTab()?.isLoading ? "tabs.stop" : "tabs.reload", { tabId: browserState.activeTabId })}
      >
        <span aria-hidden="true">{activeTab()?.isLoading ? "×" : "↻"}</span>
      </button>
    </div>
  );
}

function ActionCluster(props: Pick<Props, "onBookmarkEdit" | "onBookmarks" | "onDownloads">) {
  const isBookmarked = () => {
    const tab = activeTab();
    return !!tab?.url && browserState.bookmarks.some((bookmark) => bookmark.url === tab.url);
  };

  const toggleBookmark = async (anchor: HTMLElement) => {
    const tab = activeTab();
    if (!tab?.url) return;
    if (isBookmarked()) {
      await fubuki.invoke("bookmarks.remove", { url: tab.url });
      await refreshState("bookmarks.changed");
      return;
    }
    props.onBookmarkEdit(anchor);
  };

  const sharePage = async () => {
    const tab = activeTab();
    const url = tab?.url;
    if (!url) return;
    const title = tab.title || "Fubuki Browser";
    try {
      if (navigator.share) {
        await navigator.share({ title, url });
      } else {
        await navigator.clipboard?.writeText(url);
      }
    } catch {
      await navigator.clipboard?.writeText(url);
    }
  };

  return (
    <div class="action-cluster">
      <button classList={{ "tool-button": true, selected: isBookmarked() }} title={isBookmarked() ? label("Remove bookmark", "ブックマークを削除") : label("Add bookmark", "ブックマークを追加")} aria-label={isBookmarked() ? label("Remove bookmark", "ブックマークを削除") : label("Add bookmark", "ブックマークを追加")} onClick={(event) => void toggleBookmark(event.currentTarget)}>
        <span aria-hidden="true">{isBookmarked() ? "★" : "☆"}</span>
      </button>
      <button class="tool-button" title={label("Copy or share link", "リンクを共有/コピー")} aria-label={label("Copy or share link", "リンクを共有/コピー")} disabled={!activeTab()?.url} onClick={() => void sharePage()}>
        <span aria-hidden="true">↗</span>
      </button>
      <button class="tool-button" title={label("Bookmarks", "ブックマーク一覧")} aria-label={label("Bookmarks", "ブックマーク一覧")} onPointerDown={(event) => event.stopPropagation()} onClick={(event) => props.onBookmarks(event.currentTarget)}>
        <span aria-hidden="true">▤</span>
      </button>
      <button classList={{ "tool-button": true, attention: browserState.downloads.some((item) => item.state === "in_progress") }} title={label("Downloads", "ダウンロード")} aria-label={label("Downloads", "ダウンロード")} onPointerDown={(event) => event.stopPropagation()} onClick={(event) => props.onDownloads(event.currentTarget)}>
        <span aria-hidden="true">↓</span>
      </button>
    </div>
  );
}

function MoreMenu(props: Pick<Props, "onBookmarks" | "onHistory" | "onDownloads" | "onSettings" | "onOverlayActive">) {
  let menu: HTMLDivElement | undefined;
  const [open, setOpen] = createSignal(false);

  createEffect(() => {
    props.onOverlayActive(open());
    if (!open()) return;
    let ready = false;
    window.setTimeout(() => {
      ready = true;
    }, 0);
    const onPointerDown = (event: PointerEvent) => {
      if (!ready) return;
      if (menu && !menu.contains(event.target as Node)) setOpen(false);
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpen(false);
    };
    window.addEventListener("pointerdown", onPointerDown);
    window.addEventListener("keydown", onKeyDown);
    onCleanup(() => {
      window.removeEventListener("pointerdown", onPointerDown);
      window.removeEventListener("keydown", onKeyDown);
      props.onOverlayActive(false);
    });
  });

  const run = (action: () => void) => {
    action();
    setOpen(false);
  };

  const copyUrl = () => {
    const url = activeTab()?.url;
    if (url) void navigator.clipboard?.writeText(url);
  };

  return (
    <div ref={menu} class="more-menu">
      <button class="tool-button" title={label("More", "その他")} aria-label={label("More", "その他")} aria-haspopup="menu" aria-expanded={open()} onPointerDown={(event) => event.stopPropagation()} onClick={() => setOpen(!open())}>
        <span aria-hidden="true">•••</span>
      </button>
      <Show when={open()}>
        <div class="menu-popover" role="menu">
          <button role="menuitem" onClick={() => run(() => void fubuki.invoke("tabs.create", { active: true }))}>{label("New Tab", "新規タブ")}</button>
          <button role="menuitem" onClick={() => run(() => props.onBookmarks(menu))}>{label("Bookmarks", "ブックマーク")}</button>
          <button role="menuitem" onClick={() => run(() => props.onHistory(menu))}>{label("History", "履歴")}</button>
          <button role="menuitem" onClick={() => run(() => props.onDownloads(menu))}>{label("Downloads", "ダウンロード")}</button>
          <button role="menuitem" onClick={() => run(props.onSettings)}>{label("Settings", "設定")}</button>
          <button role="menuitem" onClick={() => run(() => void fubuki.invoke("app.openDevTools"))}>DevTools</button>
          <button role="menuitem" onClick={() => run(copyUrl)}>{label("Copy URL", "URLをコピー")}</button>
          <button role="menuitem" onClick={() => run(() => navigate("fubuki://settings/about"))}>{label("About Fubuki", "Fubukiについて")}</button>
        </div>
      </Show>
    </div>
  );
}

export default function TopBar(props: Props) {
  const [draft, setDraft] = createSignal("");

  const submit = () => {
    const tab = activeTab();
    if (tab) {
      void fubuki.invoke("tabs.navigate", { tabId: tab.id, input: draft() || tab.url });
    }
  };

  const showSidebar = () => {
    void fubuki.invoke("settings.set", { key: "sidebarVisible", value: "show" }).then(() => refreshState("settings.saved"));
  };

  return (
    <header class="top-bar" aria-label={label("Navigation", "ナビゲーション")}>
      <Show when={browserState.settings.sidebarVisible === "hide"}>
        <button class="tool-button sidebar-toggle" title={label("Show sidebar", "サイドバーを表示")} aria-label={label("Show sidebar", "サイドバーを表示")} onClick={showSidebar}>
          <span aria-hidden="true">▣</span>
        </button>
      </Show>
      <NavigationControls />
      <Omnibox value={activeTab()?.url ?? ""} onDraft={setDraft} onSubmit={submit} />
      <ActionCluster onBookmarkEdit={props.onBookmarkEdit} onBookmarks={props.onBookmarks} onDownloads={props.onDownloads} />
      <MoreMenu onBookmarks={props.onBookmarks} onHistory={props.onHistory} onDownloads={props.onDownloads} onSettings={props.onSettings} onOverlayActive={props.onOverlayActive} />
    </header>
  );
}
