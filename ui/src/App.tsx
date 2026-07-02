import { createEffect, createSignal, onCleanup, onMount } from "solid-js";
import { commands, fubuki, page, tabs } from "./bridge/fubuki";
import BrowserShell from "./components/BrowserShell";
import { bindNativeEvents, browserState, refreshState } from "./stores/browserStore";

function activeTab() {
  return browserState.tabs.find((tab) => tab.id === browserState.activeTabId);
}

function navigateInternal(url: string) {
  const tab = activeTab();
  if (tab) {
    void fubuki.invoke("tabs.navigate", { tabId: tab.id, input: url });
  } else {
    void fubuki.invoke("tabs.create", { url, active: true });
  }
}

export default function App() {
  const [systemDark, setSystemDark] = createSignal(window.matchMedia?.("(prefers-color-scheme: dark)").matches ?? false);

  createEffect(() => {
    const appearance = browserState.settings.appearance || browserState.settings.theme || "system";
    document.documentElement.dataset.theme = appearance === "dark" || (appearance === "system" && systemDark()) ? "dark" : "light";
    document.documentElement.dataset.sidebar = browserState.settings.sidebarVisible === "hide" ? "hide" : "show";
    const width = Math.min(280, Math.max(168, Number(browserState.settings.sidebarWidth) || 196));
    document.documentElement.style.setProperty("--sidebar-width", `${width}px`);
  });

  onMount(() => {
    const disposeNativeEvents = bindNativeEvents();
    const media = window.matchMedia?.("(prefers-color-scheme: dark)");
    const onSchemeChange = () => setSystemDark(media?.matches ?? false);

    const toggleBookmark = async () => {
      const tab = activeTab();
      if (!tab?.url || tab.url.startsWith("fubuki://") || tab.url.startsWith("data:")) return;

      const bookmarked = browserState.bookmarks.some((bookmark) => bookmark.url === tab.url);
      if (bookmarked) {
        await fubuki.invoke("bookmarks.remove", { url: tab.url });
      } else {
        await fubuki.invoke("bookmarks.save", {
          title: tab.title || tab.url,
          url: tab.url,
          faviconUrl: tab.faviconUrl || ""
        });
      }
      await refreshState("bookmarks.changed");
    };

    const toggleSidebar = () => {
      const next = browserState.settings.sidebarVisible === "hide" ? "show" : "hide";
      void fubuki.invoke("settings.set", { key: "sidebarVisible", value: next }).then(() => refreshState("settings.saved"));
    };

    const onKeyDown = (event: KeyboardEvent) => {
      const command = event.metaKey || event.ctrlKey;
      if (!command) return;

      const tab = activeTab();
      const key = event.key.toLowerCase();
      if (key === "l") {
        const input = document.querySelector<HTMLInputElement>(".omnibox-input");
        input?.focus();
        input?.select();
        event.preventDefault();
        return;
      }
      if (key === "n" && event.shiftKey) {
        void commands.execute("windows.createPrivate");
        event.preventDefault();
        return;
      }
      if (key === "n") {
        void commands.execute("windows.create");
        event.preventDefault();
        return;
      }
      if (key === "t" && event.shiftKey) {
        void tabs.reopenClosed();
        event.preventDefault();
        return;
      }
      if (key === "t") {
        void tabs.create();
        event.preventDefault();
        return;
      }
      if (key === "f") {
        window.dispatchEvent(new CustomEvent("fubuki:show-find"));
        event.preventDefault();
        return;
      }
      if (event.key === "+" || event.key === "=") {
        void page.zoomIn();
        event.preventDefault();
        return;
      }
      if (event.key === "-") {
        void page.zoomOut();
        event.preventDefault();
        return;
      }
      if (event.key === "0") {
        void page.zoomReset();
        event.preventDefault();
        return;
      }
      if (key === "b") {
        toggleSidebar();
        event.preventDefault();
        return;
      }
      if (event.key === ",") {
        navigateInternal("fubuki://settings/");
        event.preventDefault();
        return;
      }
      if (key === "d") {
        void toggleBookmark();
        event.preventDefault();
        return;
      }
      if (!tab) return;
      if (key === "w" && event.shiftKey) {
        void commands.execute("windows.close");
        event.preventDefault();
      } else if (key === "w") {
        void fubuki.invoke("tabs.close", { tabId: tab.id });
        event.preventDefault();
      } else if (key === "r") {
        void fubuki.invoke("tabs.reload", { tabId: tab.id });
        event.preventDefault();
      } else if (event.key === "[") {
        void fubuki.invoke("tabs.goBack", { tabId: tab.id });
        event.preventDefault();
      } else if (event.key === "]") {
        void fubuki.invoke("tabs.goForward", { tabId: tab.id });
        event.preventDefault();
      }
    };

    const onToggleActiveBookmark = () => void toggleBookmark();

    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("fubuki:toggle-active-bookmark", onToggleActiveBookmark);
    window.addEventListener("fubuki:toggle-sidebar", toggleSidebar);
    media?.addEventListener("change", onSchemeChange);

    onCleanup(() => {
      disposeNativeEvents();
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("fubuki:toggle-active-bookmark", onToggleActiveBookmark);
      window.removeEventListener("fubuki:toggle-sidebar", toggleSidebar);
      media?.removeEventListener("change", onSchemeChange);
    });
  });

  return <BrowserShell />;
}
