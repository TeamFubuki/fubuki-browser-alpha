import { createEffect, onCleanup, onMount } from "solid-js";
import { bindNativeEvents, browserState, refreshState } from "./stores/browserStore";
import TabStrip from "./components/TabStrip";
import Toolbar from "./components/Toolbar";
import { fubuki, fubukiLogoDataUri } from "./bridge/fubuki";

export default function App() {
  createEffect(() => {
    document.documentElement.dataset.theme = browserState.settings.theme || "light";
  });

  onMount(() => {
    const dispose = bindNativeEvents();
    const onKeyDown = (event: KeyboardEvent) => {
      const command = event.metaKey || event.ctrlKey;
      const activeTabId = browserState.activeTabId;
      if (!activeTabId) return;
      if (command && event.key.toLowerCase() === "l") {
        document.querySelector<HTMLInputElement>(".omnibox input")?.select();
        event.preventDefault();
      } else if (command && event.key.toLowerCase() === "r") {
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
        const tab = browserState.tabs.find((item) => item.id === activeTabId);
        const isBookmarked = !!tab?.url && browserState.bookmarks.some((bookmark) => bookmark.url === tab.url);
        void fubuki.invoke(isBookmarked ? "bookmarks.remove" : "bookmarks.addActive", isBookmarked ? { url: tab?.url } : undefined).then(() => refreshState("bookmarks.changed"));
        event.preventDefault();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    onCleanup(dispose);
    onCleanup(() => window.removeEventListener("keydown", onKeyDown));
  });

  return (
    <main class="app-shell">
      <img class="app-logo" src={fubukiLogoDataUri} alt="Fubuki Browser" />
      <TabStrip />
      <Toolbar />
    </main>
  );
}
