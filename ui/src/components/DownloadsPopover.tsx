import { For, Show, createEffect, onCleanup } from "solid-js";
import type { PanelAnchor } from "../App";
import { fubuki } from "../bridge/fubuki";
import { browserState, refreshState } from "../stores/browserStore";

type Props = {
  open: boolean;
  anchor?: PanelAnchor;
  onClose: () => void;
};

function fileName(path?: string, url?: string) {
  const value = path || url || "";
  const parts = value.split(/[\\/]/);
  return parts.at(-1) || value || "Download";
}

function label(en: string, ja: string) {
  return browserState.settings.language === "ja" ? ja : en;
}

function shortPath(path?: string) {
  if (!path) return "";
  const parts = path.split(/[\\/]/).filter(Boolean);
  if (parts.length <= 2) return path;
  return `.../${parts.slice(-2).join("/")}`;
}

function anchorStyle(anchor?: PanelAnchor) {
  return anchor ? `--popover-top: ${anchor.top}px; --popover-right: ${anchor.right}px;` : undefined;
}

export default function DownloadsPopover(props: Props) {
  let panel: HTMLElement | undefined;

  createEffect(() => {
    if (!props.open) return;
    let ready = false;
    window.setTimeout(() => {
      ready = true;
    }, 0);
    const onPointerDown = (event: PointerEvent) => {
      if (!ready) return;
      if (panel && !panel.contains(event.target as Node)) props.onClose();
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") props.onClose();
    };
    window.addEventListener("pointerdown", onPointerDown);
    window.addEventListener("keydown", onKeyDown);
    onCleanup(() => {
      window.removeEventListener("pointerdown", onPointerDown);
      window.removeEventListener("keydown", onKeyDown);
    });
  });

  return (
    <Show when={props.open}>
      <section ref={panel} class="popover downloads-popover" style={anchorStyle(props.anchor)} aria-label="Downloads">
        <header>
          <h2>{label("Downloads", "ダウンロード")}</h2>
          <button
            class="mini-action"
            onClick={() => void fubuki.invoke("data.clear", { target: "downloads" }).then(() => refreshState("downloads.cleared"))}
          >
            {label("Clear", "消去")}
          </button>
        </header>
        <Show when={browserState.downloads.length > 0} fallback={<p class="empty-state">{label("No downloads", "ダウンロードはありません")}</p>}>
          <div class="popover-list">
            <For each={browserState.downloads.slice(0, 20)}>
              {(item) => (
                <article class="download-row" title={item.path || item.url}>
                  <span class="download-icon" aria-hidden="true">↓</span>
                  <span>{fileName(item.path, item.url)}</span>
                  <span class="download-state">{item.state || "unknown"}</span>
                  <progress value={item.percent ?? 0} max="100" aria-label={label("Download progress", "ダウンロード進捗")} />
                  <small>{item.percent ?? 0}% · {shortPath(item.path) || fileName(undefined, item.url)}</small>
                </article>
              )}
            </For>
          </div>
        </Show>
      </section>
    </Show>
  );
}
