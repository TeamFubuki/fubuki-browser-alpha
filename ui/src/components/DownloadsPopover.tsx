import { For, Show, createEffect, onCleanup } from "solid-js";
import { fubuki } from "../bridge/fubuki";
import { browserState, refreshState } from "../stores/browserStore";

type Props = {
  open: boolean;
  onClose: () => void;
};

function fileName(path?: string, url?: string) {
  const value = path || url || "";
  const parts = value.split(/[\\/]/);
  return parts.at(-1) || value || "Download";
}

export default function DownloadsPopover(props: Props) {
  let panel: HTMLElement | undefined;

  createEffect(() => {
    if (!props.open) return;
    const onPointerDown = (event: PointerEvent) => {
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
      <section ref={panel} class="popover downloads-popover" aria-label="Downloads">
        <header>
          <h2>Downloads</h2>
          <button
            class="mini-action"
            onClick={() => void fubuki.invoke("data.clear", { target: "downloads" }).then(() => refreshState("downloads.cleared"))}
          >
            Clear
          </button>
        </header>
        <Show when={browserState.downloads.length > 0} fallback={<p class="empty-state">No downloads</p>}>
          <div class="popover-list">
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
    </Show>
  );
}
