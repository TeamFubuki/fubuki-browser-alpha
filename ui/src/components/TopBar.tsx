import { createMemo, createSignal, onCleanup, onMount } from "solid-js";
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
  const [findText, setFindText] = createSignal('');

  const currentTab = createMemo(() => activeTab());
  const isBookmarked = createMemo(() => isTabBookmarked(currentTab()?.url));
  const isLoading = createMemo(() => currentTab()?.isLoading ?? false);
  const canGoBack = createMemo(() => currentTab()?.canGoBack ?? false);
  const canGoForward = createMemo(() => currentTab()?.canGoForward ?? false);
  const tabUrl = createMemo(() => currentTab()?.url);

  const isDisabledUrl = createMemo(() => {
    const url = tabUrl();
    return !url || url.startsWith("fubuki://") || url.startsWith("data:");
  });

  onMount(() => {
    const showFind = () => setFindOpen(true);
    window.addEventListener('fubuki:show-find', showFind);
    onCleanup(() => window.removeEventListener('fubuki:show-find', showFind));
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
        disabled={!canGoBack()}
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
        disabled={!canGoForward()}
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
          isLoading()
            ? t("common.stop", lang())
            : t("common.reload", lang())
        }
        aria-label={
          isLoading()
            ? t("common.stop", lang())
            : t("common.reload", lang())
        }
        disabled={!currentTab()}
        onClick={() =>
          void invokeBridge(
            isLoading() ? "tabs.stop" : "tabs.reload",
            { tabId: browserState.activeTabId }
          )
        }
      >
        <span aria-hidden="true">{isLoading() ? "×" : "↻"}</span>
      </button>
      <Omnibox />
      <button
        classList={{
          "topbar-button": true,
          bookmarked: isBookmarked(),
        }}
        title={
          isBookmarked()
            ? t("action.removeBookmark", lang())
            : t("action.addBookmark", lang())
        }
        aria-label={
          isBookmarked()
            ? t("action.removeBookmark", lang())
            : t("action.addBookmark", lang())
        }
        disabled={isDisabledUrl()}
        onClick={() => void toggleBookmark()}
      >
        <span aria-hidden="true">
          {isBookmarked() ? "★" : "☆"}
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
