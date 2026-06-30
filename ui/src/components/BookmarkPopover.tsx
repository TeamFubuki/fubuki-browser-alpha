import { For, Show, createEffect, createSignal, onCleanup } from "solid-js";
import { fubuki, type BrowserRecord } from "../bridge/fubuki";
import { browserState, refreshState } from "../stores/browserStore";

type Props = {
  open: boolean;
  mode: "list" | "edit";
  bookmark?: BrowserRecord;
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

export default function BookmarkPopover(props: Props) {
  let panel: HTMLElement | undefined;
  const [title, setTitle] = createSignal("");
  const [url, setUrl] = createSignal("");
  const [originalUrl, setOriginalUrl] = createSignal("");

  createEffect(() => {
    if (!props.open) return;
    const source = props.bookmark || activeBookmark();
    setTitle(source.title || source.url || "");
    setUrl(source.url || "");
    setOriginalUrl(source.url || "");
  });

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

  return (
    <Show when={props.open}>
      <section ref={panel} class="popover bookmark-popover" aria-label="Bookmarks">
        <Show
          when={props.mode === "edit"}
          fallback={
            <>
              <header>
                <h2>Bookmarks</h2>
                <button
                  class="mini-action"
                  onClick={() => void fubuki.invoke("data.clear", { target: "bookmarks" }).then(() => refreshState("bookmarks.cleared"))}
                >
                  Clear
                </button>
              </header>
              <Show when={browserState.bookmarks.length > 0} fallback={<p class="empty-state">No bookmarks</p>}>
                <div class="popover-list">
                  <For each={browserState.bookmarks}>
                    {(item) => (
                      <button class="popover-row" title={item.url} onClick={() => void openBookmark(item)}>
                        <Show when={browserState.settings.showBookmarkFavicons !== "off"}>
                          <Show when={item.faviconUrl} fallback={<span class="record-favicon" />}>
                            <img class="record-favicon" src={item.faviconUrl} alt="" />
                          </Show>
                        </Show>
                        <span>{item.title || item.url || "Untitled"}</span>
                      </button>
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
            <header><h2>Bookmark</h2></header>
            <label>
              <span>Title</span>
              <input value={title()} onInput={(event) => setTitle(event.currentTarget.value)} />
            </label>
            <label>
              <span>URL</span>
              <input value={url()} onInput={(event) => setUrl(event.currentTarget.value)} />
            </label>
            <div class="form-actions">
              <button type="button" onClick={props.onClose}>Cancel</button>
              <button type="submit">Save</button>
            </div>
          </form>
        </Show>
      </section>
    </Show>
  );
}
