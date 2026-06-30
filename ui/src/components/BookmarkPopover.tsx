import { For, Show, createEffect, createMemo, createSignal, onCleanup } from "solid-js";
import type { PanelAnchor } from "../App";
import { fubuki, type BrowserRecord } from "../bridge/fubuki";
import { browserState, refreshState } from "../stores/browserStore";

type Props = {
  open: boolean;
  mode: "list" | "edit";
  bookmark?: BrowserRecord;
  anchor?: PanelAnchor;
  onClose: () => void;
};

function activeBookmark(): BrowserRecord {
  const tab = browserState.tabs.find((item) => item.id === browserState.activeTabId);
  return {
    title: tab?.title || "",
    url: tab?.url || "",
    faviconUrl: tab?.faviconUrl || "",
    createdAt: ""
  };
}

function anchorStyle(anchor?: PanelAnchor) {
  return anchor ? `--popover-top: ${anchor.top}px; --popover-right: ${anchor.right}px;` : undefined;
}

function label(en: string, ja: string) {
  return browserState.settings.language === "ja" ? ja : en;
}

export default function BookmarkPopover(props: Props) {
  let panel: HTMLElement | undefined;
  const [title, setTitle] = createSignal("");
  const [url, setUrl] = createSignal("");
  const [originalUrl, setOriginalUrl] = createSignal("");
  const [query, setQuery] = createSignal("");
  const [listEditing, setListEditing] = createSignal(false);

  createEffect(() => {
    if (!props.open) {
      setListEditing(false);
      return;
    }
    const source = props.bookmark || activeBookmark();
    setTitle(source.title || source.url || "");
    setUrl(source.url || "");
    setOriginalUrl(source.url || "");
  });

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

  const save = async () => {
    const nextUrl = url().trim();
    if (!nextUrl) return;
    if (originalUrl() && originalUrl() !== nextUrl) {
      await fubuki.invoke("bookmarks.remove", { url: originalUrl() });
    }
    await fubuki.invoke("bookmarks.save", { title: title().trim() || nextUrl, url: nextUrl, faviconUrl: props.bookmark?.faviconUrl || activeBookmark().faviconUrl || "" });
    await refreshState("bookmark.saved");
    props.onClose();
  };

  const openBookmark = async (item: BrowserRecord) => {
    if (!item.url) return;
    if (browserState.settings.openBookmarkIn === "new") {
      await fubuki.invoke("tabs.create", { url: item.url, active: true });
      props.onClose();
      return;
    }
    if (!browserState.activeTabId) return;
    await fubuki.invoke("tabs.navigate", { tabId: browserState.activeTabId, input: item.url });
    props.onClose();
  };

  const filteredBookmarks = createMemo(() => {
    const needle = query().trim().toLowerCase();
    if (!needle) return browserState.bookmarks;
    return browserState.bookmarks.filter((item) => `${item.title ?? ""} ${item.url ?? ""}`.toLowerCase().includes(needle));
  });

  const removeBookmark = async (event: MouseEvent, item: BrowserRecord) => {
    event.stopPropagation();
    if (!item.url) return;
    await fubuki.invoke("bookmarks.remove", { url: item.url });
    await refreshState("bookmarks.changed");
  };

  const editFromList = (item: BrowserRecord) => {
    setTitle(item.title || item.url || "");
    setUrl(item.url || "");
    setOriginalUrl(item.url || "");
    setListEditing(true);
  };

  return (
    <Show when={props.open}>
      <section ref={panel} class="popover bookmark-popover" style={anchorStyle(props.anchor)} aria-label="Bookmarks">
        <Show
          when={props.mode === "edit" || listEditing()}
          fallback={
            <>
              <header>
                <h2>{label("Bookmarks", "ブックマーク")}</h2>
                <button
                  class="mini-action"
                  onClick={() => void fubuki.invoke("data.clear", { target: "bookmarks" }).then(() => refreshState("bookmarks.cleared"))}
                >
                  {label("Clear", "消去")}
                </button>
              </header>
              <input class="panel-search" value={query()} placeholder={label("Search bookmarks", "ブックマークを検索")} aria-label={label("Search bookmarks", "ブックマークを検索")} onInput={(event) => setQuery(event.currentTarget.value)} />
              <Show when={filteredBookmarks().length > 0} fallback={<p class="empty-state">{label("No bookmarks", "ブックマークはありません")}</p>}>
                <div class="popover-list">
                  <For each={filteredBookmarks()}>
                    {(item) => (
                      <div class="record-line">
                        <button class="popover-row rich-row" title={item.url} onClick={() => void openBookmark(item)}>
                          <Show when={browserState.settings.showBookmarkFavicons !== "off"}>
                            <Show when={item.faviconUrl} fallback={<span class="record-favicon" />}>
                              <img class="record-favicon" src={item.faviconUrl} alt="" />
                            </Show>
                          </Show>
                          <span>{item.title || item.url || "Untitled"}</span>
                          <small>{item.url}</small>
                        </button>
                        <button class="row-action" title={label("Edit", "編集")} aria-label={label("Edit", "編集")} onClick={(event) => {
                          event.stopPropagation();
                          editFromList(item);
                        }}>✎</button>
                        <button class="row-action" title={label("Delete", "削除")} aria-label={label("Delete", "削除")} onClick={(event) => void removeBookmark(event, item)}>×</button>
                      </div>
                    )}
                  </For>
                </div>
              </Show>
            </>
          }
        >
          <form
            class="bookmark-form"
            onSubmit={(event) => {
              event.preventDefault();
              void save();
            }}
          >
            <header><h2>{label("Bookmark", "ブックマーク")}</h2></header>
            <label>
              <span>{label("Title", "タイトル")}</span>
              <input value={title()} onInput={(event) => setTitle(event.currentTarget.value)} />
            </label>
            <label>
              <span>URL</span>
              <input value={url()} onInput={(event) => setUrl(event.currentTarget.value)} />
            </label>
            <div class="form-actions">
              <Show when={originalUrl()}>
                <button type="button" onClick={() => void fubuki.invoke("bookmarks.remove", { url: originalUrl() }).then(() => refreshState("bookmarks.changed")).then(props.onClose)}>{label("Delete", "削除")}</button>
              </Show>
              <button type="button" onClick={props.onClose}>{label("Cancel", "キャンセル")}</button>
              <button type="submit">{label("Save", "保存")}</button>
            </div>
          </form>
        </Show>
      </section>
    </Show>
  );
}
