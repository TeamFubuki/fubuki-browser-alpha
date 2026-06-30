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

function pathToFileUrl(path?: string) {
  if (!path) return "";
  return `file://${path.split("/").map(encodeURIComponent).join("/")}`;
}

function anchorStyle(anchor?: PanelAnchor) {
  return anchor ? `--popover-top: ${anchor.top}px; --popover-right: ${anchor.right}px;` : undefined;
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

  const openDownload = async (item: { path?: string; url?: string }) => {
    const target = pathToFileUrl(item.path) || item.url;
    if (!target) return;
    if (browserState.activeTabId) {
      await fubuki.invoke("tabs.navigate", { tabId: browserState.activeTabId, input: target });
    } else {
      await fubuki.invoke("tabs.create", { url: target, active: true });
    }
    props.onClose();
  };

  const removeDownload = async (item: { path?: string; url?: string }) => {
    await fubuki.invoke("downloads.remove", { url: item.url || "", path: item.path || "" });
    await refreshState("downloads.removed");
  };

  return (
    <Show when={props.open}>
      <section ref={panel} class="popover downloads-popover" style={anchorStyle(props.anchor)} aria-label="Downloads">
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
                  <progress value={item.percent ?? 0} max="100" aria-label="Download progress" />
                  <small>{item.state || "unknown"} · {item.percent ?? 0}% · {item.path || item.url}</small>
                  <div class="download-actions">
                    <button class="mini-action" onClick={() => void openDownload(item)}>Open</button>
                    <button class="mini-action" onClick={() => void removeDownload(item)}>Delete</button>
                  </div>
                </article>
              )}
            </For>
          </div>
        </Show>
      </section>
    </Show>
  );
}
