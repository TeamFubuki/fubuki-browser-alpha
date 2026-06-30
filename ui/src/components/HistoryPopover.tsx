import { For, Show, createEffect, createMemo, createSignal, onCleanup } from "solid-js";
import type { PanelAnchor } from "../App";
import { fubuki, type BrowserRecord } from "../bridge/fubuki";
import { browserState, refreshState } from "../stores/browserStore";

type Props = {
  open: boolean;
  anchor?: PanelAnchor;
  onClose: () => void;
};

function dayLabel(value?: string) {
  if (!value) return "Earlier";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "Earlier";
  return new Intl.DateTimeFormat(undefined, { dateStyle: "medium" }).format(date);
}

function groupHistory(items: BrowserRecord[]) {
  const groups: Array<{ day: string; items: BrowserRecord[] }> = [];
  for (const item of items) {
    const day = dayLabel(item.createdAt);
    const existing = groups.find((group) => group.day === day);
    if (existing) {
      existing.items.push(item);
    } else {
      groups.push({ day, items: [item] });
    }
  }
  return groups;
}

function anchorStyle(anchor?: PanelAnchor) {
  return anchor ? `--popover-top: ${anchor.top}px; --popover-right: ${anchor.right}px;` : undefined;
}

export default function HistoryPopover(props: Props) {
  let panel: HTMLElement | undefined;
  const [query, setQuery] = createSignal("");

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

  const filtered = createMemo(() => {
    const needle = query().trim().toLowerCase();
    if (!needle) return browserState.history;
    return browserState.history.filter((item) => `${item.title ?? ""} ${item.url ?? ""}`.toLowerCase().includes(needle));
  });

  const openHistory = async (item: BrowserRecord) => {
    if (!item.url) return;
    if (browserState.activeTabId) {
      await fubuki.invoke("tabs.navigate", { tabId: browserState.activeTabId, input: item.url });
    } else {
      await fubuki.invoke("tabs.create", { url: item.url, active: true });
    }
    props.onClose();
  };

  const removeHistory = async (event: MouseEvent, item: BrowserRecord) => {
    event.stopPropagation();
    if (!item.url) return;
    await fubuki.invoke("history.remove", { url: item.url });
    await refreshState("history.removed");
  };

  return (
    <Show when={props.open}>
      <section ref={panel} class="popover history-popover" style={anchorStyle(props.anchor)} aria-label="History">
        <header>
          <h2>History</h2>
          <button class="mini-action" onClick={() => void fubuki.invoke("data.clear", { target: "history" }).then(() => refreshState("history.cleared"))}>Clear</button>
        </header>
        <input class="panel-search" value={query()} placeholder="Search history" aria-label="Search history" onInput={(event) => setQuery(event.currentTarget.value)} />
        <Show when={filtered().length > 0} fallback={<p class="empty-state">No history</p>}>
          <div class="popover-list grouped-list">
            <For each={groupHistory(filtered())}>
              {(group) => (
                <section class="record-group">
                  <h3>{group.day}</h3>
                  <For each={group.items}>
                    {(item) => (
                      <div class="record-line">
                        <button class="popover-row rich-row" title={item.url} onClick={() => void openHistory(item)}>
                          <span class="record-favicon" aria-hidden="true" />
                          <span>{item.title || item.url || "Untitled"}</span>
                          <small>{item.url}</small>
                        </button>
                        <button class="row-action" title="Delete" aria-label="Delete" onClick={(event) => void removeHistory(event, item)}>×</button>
                      </div>
                    )}
                  </For>
                </section>
              )}
            </For>
          </div>
        </Show>
      </section>
    </Show>
  );
}
