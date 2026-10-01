import { createMemo, createSignal, For, Show } from "solid-js";
import { downloadState, formatTime, safeLink, type PageData, type Page } from "../data";
import { Action, type Translate } from "../components";

export function Records(props: { page: Page; data: PageData; t: Translate }) {
  const [search, setSearch] = createSignal("");
  const records = createMemo(() =>
    props.data.records.filter((record) =>
      `${record.title} ${record.url}`.toLowerCase().includes(search().toLowerCase()),
    ),
  );
  return (
    <>
      <Show when={props.page === "history"}>
        <input
          class="toolbar"
          type="search"
          aria-label={props.t("Search history")}
          placeholder={props.t("Search history")}
          value={search()}
          onInput={(event) => setSearch(event.currentTarget.value)}
        />
        <div class="segmented toolbar">
          <For
            each={[
              ["lastHour", "Clear last hour"],
              ["today", "Clear today"],
              ["all", "Clear all"],
            ]}
          >
            {([value, label]) => (
              <Action
                page="history"
                params={{ key: "clearHistoryRange", value }}
                label={props.t(label)}
                danger
              />
            )}
          </For>
        </div>
      </Show>
      <Show when={props.page === "downloads"}>
        <div class="toolbar">
          <Action
            page="downloads"
            params={{ key: "clearData", value: "downloads" }}
            label={props.t("Clear downloads")}
            danger
          />
        </div>
      </Show>
      <Show
        when={records().length}
        fallback={
          <p class="empty">
            {props.t(props.data.records.length ? "No results" : `No ${props.page}`)}
          </p>
        }
      >
        <div class="list">
          <For each={records()}>
            {(record, index) => (
              <>
                <Show
                  when={
                    props.page === "history" &&
                    (index() === 0 ||
                      formatTime(records()[index() - 1].createdAt, true) !==
                        formatTime(record.createdAt, true))
                  }
                >
                  <h2>{formatTime(record.createdAt, true) || props.t("Earlier")}</h2>
                </Show>
                <article class="row">
                  <span class="favicon">
                    <Show when={safeLink(record.faviconUrl)}>
                      {(url) => <img src={url()} alt="" />}
                    </Show>
                  </span>
                  <Show
                    when={props.page === "downloads"}
                    fallback={
                      <a href={safeLink(record.url)} title={record.url}>
                        <span class="title">{record.title || record.url}</span>
                        <span class="meta">
                          {props.page === "history" ? `${formatTime(record.createdAt)} · ` : ""}
                          {record.url}
                        </span>
                      </a>
                    }
                  >
                    <div class="download-main">
                      <div>
                        <span class="title">
                          {(record.path || record.url).split(/[\\/]/).pop()}
                        </span>
                        <span class="meta">{record.path || record.url}</span>
                      </div>
                      <div class="download-status">
                        <Show when={["started", "in_progress"].includes(downloadState(record))}>
                          <progress
                            max="100"
                            value={Math.max(0, Math.min(100, record.percent))}
                            aria-label={props.t("Downloading")}
                          />
                        </Show>
                        <span>
                          {props.t(
                            (
                              {
                                completed: "Completed",
                                canceled: "Canceled",
                                failed: "Failed",
                                started: "Starting",
                                in_progress: "Downloading",
                              } as Record<string, string>
                            )[downloadState(record)] || downloadState(record),
                          )}
                          {["started", "in_progress"].includes(downloadState(record))
                            ? ` ${Math.max(0, Math.min(100, record.percent))}%`
                            : ""}
                        </span>
                      </div>
                    </div>
                  </Show>
                  <div class="download-actions">
                    <Show when={props.page === "downloads"}>
                      <Action
                        page="downloads"
                        params={{ key: "openDownload", value: record.path }}
                        label={props.t("Open")}
                        disabled={!record.path}
                      />
                      <Action
                        page="downloads"
                        params={{ key: "revealDownload", value: record.path }}
                        label={props.t("Reveal")}
                        disabled={!record.path}
                      />
                    </Show>
                    <Action
                      page={props.page}
                      params={{
                        key:
                          props.page === "history"
                            ? "removeHistory"
                            : props.page === "bookmarks"
                              ? "removeBookmark"
                              : "removeDownload",
                        value: props.page === "downloads" ? record.downloadId : record.url,
                      }}
                      label={props.t("Delete")}
                      danger
                    />
                  </div>
                </article>
              </>
            )}
          </For>
        </div>
      </Show>
    </>
  );
}
