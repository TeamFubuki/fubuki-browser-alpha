import { createEffect, createSignal, onCleanup, onMount } from "solid-js";
import { bindNativeEvents, browserState, refreshState } from "./stores/browserStore";
import AppShell from "./components/AppShell";
import BookmarkPopover from "./components/BookmarkPopover";
import DownloadsPopover from "./components/DownloadsPopover";
import HistoryPopover from "./components/HistoryPopover";
import PanelLayer from "./components/PanelLayer";
import Sidebar from "./components/Sidebar";
import TopBar from "./components/TopBar";
import WebViewArea from "./components/WebViewArea";
import { fubuki, fubukiLogoDataUri, type BrowserRecord } from "./bridge/fubuki";

export type PanelAnchor = {
  top: number;
  right: number;
};

function anchorFromElement(element?: HTMLElement): PanelAnchor | undefined {
  if (!element) return undefined;
  const rect = element.getBoundingClientRect();
  return {
    top: Math.round(rect.bottom + 8),
    right: Math.round(window.innerWidth - rect.right)
  };
}

export default function App() {
  const [bookmarkPanel, setBookmarkPanel] = createSignal<"closed" | "list" | "edit">("closed");
  const [editingBookmark, setEditingBookmark] = createSignal<BrowserRecord | undefined>();
  const [panel, setPanel] = createSignal<"closed" | "bookmarks" | "history" | "downloads">("closed");
  const [panelAnchor, setPanelAnchor] = createSignal<PanelAnchor | undefined>();
  const [systemDark, setSystemDark] = createSignal(window.matchMedia?.("(prefers-color-scheme: dark)").matches ?? false);

  const openSettings = () => {
    if (browserState.activeTabId) {
      void fubuki.invoke("tabs.navigate", { tabId: browserState.activeTabId, input: "fubuki://settings/" });
    } else {
      void fubuki.invoke("tabs.create", { url: "fubuki://settings/", active: true });
    }
  };

  createEffect(() => {
    const appearance = browserState.settings.appearance || browserState.settings.theme || "system";
    document.documentElement.dataset.theme = appearance === "dark" || (appearance === "system" && systemDark()) ? "dark" : "light";
    document.documentElement.dataset.density = browserState.settings.toolbarDensity || "compact";
    const sidebarState = browserState.settings.sidebarVisible || "show";
    const sidebarWidth = sidebarState === "collapsed" ? 54 : Math.min(240, Math.max(160, Number(browserState.settings.sidebarWidth) || 196));
    document.documentElement.style.setProperty("--sidebar-width", `${sidebarWidth}px`);
    document.documentElement.dataset.sidebar = browserState.settings.sidebarVisible || "show";
  });

  createEffect(() => {
    const active = bookmarkPanel() !== "closed" || panel() !== "closed";
    void fubuki.invoke("ui.setOverlayActive", { active }).catch(() => undefined);
  });

  onMount(() => {
    const dispose = bindNativeEvents();
    const media = window.matchMedia?.("(prefers-color-scheme: dark)");
    const onSchemeChange = () => setSystemDark(media?.matches ?? false);
    const toggleBookmarks = () => {
      openBookmarks();
    };
    const toggleActiveBookmark = () => {
      const activeTabId = browserState.activeTabId;
      const tab = browserState.tabs.find((item) => item.id === activeTabId);
      const isBookmarked = !!tab?.url && browserState.bookmarks.some((bookmark) => bookmark.url === tab.url);
      if (isBookmarked) {
        void fubuki.invoke("bookmarks.remove", { url: tab?.url }).then(() => refreshState("bookmarks.changed"));
      } else {
        setEditingBookmark(undefined);
        setBookmarkPanel("edit");
      }
    };
    const onKeyDown = (event: KeyboardEvent) => {
      const command = event.metaKey || event.ctrlKey;
      const activeTabId = browserState.activeTabId;
      if (command && event.key.toLowerCase() === "l") {
        document.querySelector<HTMLInputElement>(".omnibox input")?.select();
        event.preventDefault();
      } else if (command && event.key.toLowerCase() === "b") {
        const next = browserState.settings.sidebarVisible === "collapsed" ? "show" : "collapsed";
        void fubuki.invoke("settings.set", { key: "sidebarVisible", value: next }).then(() => refreshState("settings.saved"));
        event.preventDefault();
      } else if (command && event.key === ",") {
        openSettings();
        event.preventDefault();
      } else if (event.key === "Escape") {
        setBookmarkPanel("closed");
        setPanel("closed");
        setPanelAnchor(undefined);
      }
      if (!activeTabId) return;
      if (command && event.key.toLowerCase() === "r") {
        void fubuki.invoke("tabs.reload", { tabId: activeTabId });
        event.preventDefault();
      } else if ((command && event.key === "[") || (event.altKey && event.key === "ArrowLeft")) {
        void fubuki.invoke("tabs.goBack", { tabId: activeTabId });
        event.preventDefault();
      } else if ((command && event.key === "]") || (event.altKey && event.key === "ArrowRight")) {
        void fubuki.invoke("tabs.goForward", { tabId: activeTabId });
        event.preventDefault();
      } else if (command && event.key.toLowerCase() === "t") {
        void fubuki.invoke("tabs.create", { active: true });
        event.preventDefault();
      } else if (command && event.key.toLowerCase() === "w") {
        void fubuki.invoke("tabs.close", { tabId: activeTabId });
        event.preventDefault();
      } else if (command && event.key.toLowerCase() === "d") {
        toggleActiveBookmark();
        event.preventDefault();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    media?.addEventListener("change", onSchemeChange);
    window.addEventListener("fubuki:toggle-bookmarks", toggleBookmarks);
    window.addEventListener("fubuki:toggle-active-bookmark", toggleActiveBookmark);
    onCleanup(dispose);
    onCleanup(() => {
      window.removeEventListener("keydown", onKeyDown);
      media?.removeEventListener("change", onSchemeChange);
      window.removeEventListener("fubuki:toggle-bookmarks", toggleBookmarks);
      window.removeEventListener("fubuki:toggle-active-bookmark", toggleActiveBookmark);
    });
  });

  const closePanels = () => {
    setBookmarkPanel("closed");
    setPanel("closed");
    setPanelAnchor(undefined);
  };

  const editBookmark = (bookmark?: BrowserRecord, anchor?: HTMLElement) => {
    setEditingBookmark(bookmark);
    setPanel("closed");
    setPanelAnchor(anchorFromElement(anchor));
    setBookmarkPanel("edit");
  };

  const openBookmarks = (anchor?: HTMLElement) => {
    setEditingBookmark(undefined);
    setPanel("closed");
    const closing = bookmarkPanel() === "list";
    setPanelAnchor(closing ? undefined : anchorFromElement(anchor));
    setBookmarkPanel(closing ? "closed" : "list");
  };

  const openPanel = (next: "history" | "downloads", anchor?: HTMLElement) => {
    setBookmarkPanel("closed");
    const closing = panel() === next;
    setPanelAnchor(closing ? undefined : anchorFromElement(anchor));
    setPanel(closing ? "closed" : next);
  };

  return (
    <AppShell>
      <img class="app-logo" src={fubukiLogoDataUri} alt="Fubuki Browser" />
      <Sidebar
        onBookmarks={() => openBookmarks()}
        onHistory={() => openPanel("history")}
        onDownloads={() => openPanel("downloads")}
        onSettings={openSettings}
      />
      <TopBar
        onBookmarkEdit={(anchor) => editBookmark(undefined, anchor)}
        onBookmarks={openBookmarks}
        onHistory={(anchor) => openPanel("history", anchor)}
        onDownloads={(anchor) => openPanel("downloads", anchor)}
        onSettings={openSettings}
      />
      <WebViewArea />
      <PanelLayer>
        <BookmarkPopover open={bookmarkPanel() !== "closed"} mode={bookmarkPanel() === "edit" ? "edit" : "list"} bookmark={editingBookmark()} anchor={panelAnchor()} onClose={closePanels} />
        <HistoryPopover open={panel() === "history"} anchor={panelAnchor()} onClose={closePanels} />
        <DownloadsPopover open={panel() === "downloads"} anchor={panelAnchor()} onClose={closePanels} />
      </PanelLayer>
    </AppShell>
  );
}
