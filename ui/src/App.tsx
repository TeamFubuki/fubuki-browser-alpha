import { createEffect, createSignal, onCleanup, onMount } from "solid-js";
import { bindNativeEvents, browserState, refreshState } from "./stores/browserStore";
import Toolbar from "./components/Toolbar";
import BookmarkPopover from "./components/BookmarkPopover";
import DownloadsPopover from "./components/DownloadsPopover";
import TabStrip from "./components/TabStrip";
import { fubuki, fubukiLogoDataUri, type BrowserRecord } from "./bridge/fubuki";

export default function App() {
  const [bookmarkPanel, setBookmarkPanel] = createSignal<"closed" | "list" | "edit">("closed");
  const [editingBookmark, setEditingBookmark] = createSignal<BrowserRecord | undefined>();
  const [downloadsOpen, setDownloadsOpen] = createSignal(false);
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
    document.documentElement.style.setProperty("--sidebar-width", `${Math.min(240, Math.max(160, Number(browserState.settings.sidebarWidth) || 196))}px`);
    document.documentElement.dataset.sidebar = browserState.settings.sidebarVisible || "show";
  });

  createEffect(() => {
    const active = bookmarkPanel() !== "closed" || downloadsOpen();
    void fubuki.invoke("ui.setOverlayActive", { active }).catch(() => undefined);
  });

  onMount(() => {
    const dispose = bindNativeEvents();
    const media = window.matchMedia?.("(prefers-color-scheme: dark)");
    const onSchemeChange = () => setSystemDark(media?.matches ?? false);
    const toggleBookmarks = () => {
      setBookmarkPanel(bookmarkPanel() === "list" ? "closed" : "list");
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
        const next = browserState.settings.sidebarVisible === "hide" ? "show" : "hide";
        void fubuki.invoke("settings.set", { key: "sidebarVisible", value: next }).then(() => refreshState("settings.saved"));
        event.preventDefault();
      } else if (command && event.key === ",") {
        openSettings();
        event.preventDefault();
      } else if (event.key === "Escape") {
        setBookmarkPanel("closed");
        setDownloadsOpen(false);
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

  const editBookmark = (bookmark?: BrowserRecord) => {
    setEditingBookmark(bookmark);
    setBookmarkPanel("edit");
  };

  return (
    <main class="app-shell">
      <img class="app-logo" src={fubukiLogoDataUri} alt="Fubuki Browser" />
      <TabStrip />
      <Toolbar
        onBookmarkEdit={() => editBookmark()}
        onBookmarks={() => setBookmarkPanel(bookmarkPanel() === "list" ? "closed" : "list")}
        onDownloads={() => setDownloadsOpen(!downloadsOpen())}
        onSettings={openSettings}
      />
      <BookmarkPopover open={bookmarkPanel() !== "closed"} mode={bookmarkPanel() === "edit" ? "edit" : "list"} bookmark={editingBookmark()} onClose={() => setBookmarkPanel("closed")} />
      <DownloadsPopover open={downloadsOpen()} onClose={() => setDownloadsOpen(false)} />
    </main>
  );
}
