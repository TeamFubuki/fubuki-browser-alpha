import { For, Show, createEffect, onCleanup } from "solid-js";
import { fubuki } from "../bridge/fubuki";
import { browserState, refreshState } from "../stores/browserStore";

type Props = {
  open: boolean;
  onClose: () => void;
};

const copy = {
  en: {
    title: "Bookmarks",
    empty: "No bookmarks",
    close: "Close",
    open: "Open bookmark",
    remove: "Remove bookmark"
  },
  ja: {
    title: "ブックマーク",
    empty: "ブックマークはありません",
    close: "閉じる",
    open: "ブックマークを開く",
    remove: "ブックマークを削除"
  }
};

function t() {
  return browserState.settings.language === "ja" ? copy.ja : copy.en;
}

export default function BookmarkModal(props: Props) {
  createEffect(() => {
    void fubuki.invoke("ui.setOverlayActive", { active: props.open }).catch(() => undefined);
  });

  onCleanup(() => {
    void fubuki.invoke("ui.setOverlayActive", { active: false }).catch(() => undefined);
  });

  const openBookmark = async (url?: string) => {
    if (!url || !browserState.activeTabId) return;
    await fubuki.invoke("tabs.navigate", { tabId: browserState.activeTabId, input: url });
    props.onClose();
  };

  const removeBookmark = async (url?: string) => {
    if (!url) return;
    await fubuki.invoke("bookmarks.remove", { url });
    await refreshState("bookmark.removed");
  };

  return (
    <Show when={props.open}>
      <div class="modal-backdrop" onClick={props.onClose}>
        <section class="bookmark-modal" aria-label={t().title} onClick={(event) => event.stopPropagation()}>
          <header>
            <h2>{t().title}</h2>
            <button class="icon-button" title={t().close} onClick={props.onClose}>
              <span aria-hidden="true">×</span>
            </button>
          </header>
          <Show when={browserState.bookmarks.length > 0} fallback={<p class="empty-state">{t().empty}</p>}>
            <div class="bookmark-list">
              <For each={browserState.bookmarks}>
                {(item) => (
                  <article class="bookmark-row">
                    <button title={t().open} onClick={() => void openBookmark(item.url)}>
                      <Show when={item.faviconUrl} fallback={<span class="record-favicon" />}>
                        <img class="record-favicon" src={item.faviconUrl} alt="" />
                      </Show>
                      <span>{item.title || item.url}</span>
                      <small>{item.url}</small>
                    </button>
                    <button class="icon-button" title={t().remove} onClick={() => void removeBookmark(item.url)}>
                      <span aria-hidden="true">−</span>
                    </button>
                  </article>
                )}
              </For>
            </div>
          </Show>
        </section>
      </div>
    </Show>
  );
}
