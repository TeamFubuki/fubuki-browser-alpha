import { For, Show, createEffect, onCleanup } from "solid-js";
import { fubuki } from "../bridge/fubuki";
import { browserState } from "../stores/browserStore";

type Props = {
  open: boolean;
  onClose: () => void;
};

const copy = {
  en: {
    title: "Downloads",
    empty: "No downloads",
    close: "Close"
  },
  ja: {
    title: "ダウンロード",
    empty: "ダウンロードはありません",
    close: "閉じる"
  }
};

function t() {
  return browserState.settings.language === "ja" ? copy.ja : copy.en;
}

function fileName(path?: string, url?: string) {
  const value = path || url || "";
  const parts = value.split(/[\\/]/);
  return parts.at(-1) || value || "Download";
}

export default function DownloadsModal(props: Props) {
  createEffect(() => {
    void fubuki.invoke("ui.setOverlayActive", { active: props.open }).catch(() => undefined);
  });

  onCleanup(() => {
    void fubuki.invoke("ui.setOverlayActive", { active: false }).catch(() => undefined);
  });

  return (
    <Show when={props.open}>
      <div class="modal-backdrop" onClick={props.onClose}>
        <section class="bookmark-modal downloads-modal" aria-label={t().title} onClick={(event) => event.stopPropagation()}>
          <header>
            <h2>{t().title}</h2>
            <button class="icon-button" title={t().close} onClick={props.onClose}>
              <span aria-hidden="true">×</span>
            </button>
          </header>
          <Show when={browserState.downloads.length > 0} fallback={<p class="empty-state">{t().empty}</p>}>
            <div class="bookmark-list">
              <For each={browserState.downloads}>
                {(item) => (
                  <article class="download-row">
                    <span class="download-icon" aria-hidden="true">↓</span>
                    <span>{fileName(item.path, item.url)}</span>
                    <small>{item.state || "unknown"} · {item.percent ?? 0}% · {item.path || item.url}</small>
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
