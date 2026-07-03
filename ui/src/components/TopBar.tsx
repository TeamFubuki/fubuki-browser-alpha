import { createSignal, onCleanup, onMount } from "solid-js";
import { invokeBridge, page } from "../bridge/fubuki";
import { t } from "../i18n";
import {
  activeTab,
  browserState,
  isTabBookmarked,
  toggleBookmark,
} from "../stores/browserStore";
import Omnibox from "./Omnibox";

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

  const lang = () => browserState.settings.language;

  return (
    <header
      classList={{ "top-bar": true, private: browserState.isPrivate }}
      aria-label="Navigation"
    >
      <button
        class="topbar-button"
        title={t("common.back", lang())}
        aria-label={t("common.back", lang())}
        disabled={!activeTab()?.canGoBack}
        onClick={() =>
          void invokeBridge("tabs.goBack", {
            tabId: browserState.activeTabId,
          })
        }
      >
        <span aria-hidden="true">←</span>
      </button>
      <button
        class="topbar-button"
        title={t("common.forward", lang())}
        aria-label={t("common.forward", lang())}
        disabled={!activeTab()?.canGoForward}
        onClick={() =>
          void invokeBridge("tabs.goForward", {
            tabId: browserState.activeTabId,
          })
        }
      >
        <span aria-hidden="true">→</span>
      </button>
      <button
        class="topbar-button"
        title={
          activeTab()?.isLoading
            ? t("common.stop", lang())
            : t("common.reload", lang())
        }
        aria-label={
          activeTab()?.isLoading
            ? t("common.stop", lang())
            : t("common.reload", lang())
        }
        disabled={!activeTab()}
        onClick={() =>
          void invokeBridge(
            activeTab()?.isLoading ? "tabs.stop" : "tabs.reload",
            { tabId: browserState.activeTabId }
          )
        }
      >
        <span aria-hidden="true">{activeTab()?.isLoading ? "×" : "↻"}</span>
      </button>
      <Omnibox />
      <button
        classList={{
          "topbar-button": true,
          bookmarked: isTabBookmarked(activeTab()?.url),
        }}
        title={
          isTabBookmarked(activeTab()?.url)
            ? t("action.removeBookmark", lang())
            : t("action.addBookmark", lang())
        }
        aria-label={
          isTabBookmarked(activeTab()?.url)
            ? t("action.removeBookmark", lang())
            : t("action.addBookmark", lang())
        }
        disabled={
          !activeTab()?.url ||
          activeTab()?.url.startsWith("fubuki://") ||
          activeTab()?.url.startsWith("data:")
        }
        onClick={() => void toggleBookmark()}
      >
        <span aria-hidden="true">
          {isTabBookmarked(activeTab()?.url) ? "★" : "☆"}
        </span>
      </button>
      {findOpen() && (
        <form
          class="find-bar"
          onSubmit={(event) => {
            event.preventDefault();
            submitFind(true);
          }}
        >
          <input
            value={findText()}
            placeholder={t("common.find", lang())}
            aria-label={t("common.find", lang())}
            onInput={(event) => setFindText(event.currentTarget.value)}
            autofocus
          />
          <button
            type="button"
            title={t("find.previous", lang())}
            onClick={() => submitFind(false)}
          >
            ↑
          </button>
          <button
            type="button"
            title={t("find.next", lang())}
            onClick={() => submitFind(true)}
          >
            ↓
          </button>
          <button
            type="button"
            title={t("action.closeFind", lang())}
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
