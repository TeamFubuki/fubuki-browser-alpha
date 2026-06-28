import { onCleanup, onMount } from "solid-js";
import { bindNativeEvents, browserState } from "./stores/browserStore";
import TabStrip from "./components/TabStrip";
import Toolbar from "./components/Toolbar";
import StatusBar from "./components/StatusBar";
import BrowserPanels from "./components/BrowserPanels";
import { fubuki } from "./bridge/fubuki";

export default function App() {
  onMount(() => {
    const dispose = bindNativeEvents();
    const onKeyDown = (event: KeyboardEvent) => {
      const mac = event.metaKey;
      const activeTabId = browserState.activeTabId;
      if (!mac || !activeTabId) return;
      if (event.key === "l") {
        document.querySelector<HTMLInputElement>(".omnibox input")?.select();
        event.preventDefault();
      } else if (event.key === "r") {
        void fubuki.invoke("tabs.reload", { tabId: activeTabId });
        event.preventDefault();
      } else if (event.key === "[") {
        void fubuki.invoke("tabs.goBack", { tabId: activeTabId });
        event.preventDefault();
      } else if (event.key === "]") {
        void fubuki.invoke("tabs.goForward", { tabId: activeTabId });
        event.preventDefault();
      } else if (event.key === "t") {
        void fubuki.invoke("tabs.create", { active: true });
        event.preventDefault();
      } else if (event.key === "w") {
        void fubuki.invoke("tabs.close", { tabId: activeTabId });
        event.preventDefault();
      } else if (event.key === "d") {
        void fubuki.invoke("bookmarks.addActive");
        event.preventDefault();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    onCleanup(dispose);
    onCleanup(() => window.removeEventListener("keydown", onKeyDown));
  });

  return (
    <main class="app-shell">
      <TabStrip />
      <Toolbar />
      <BrowserPanels />
      <StatusBar status={browserState.status} />
    </main>
  );
}
