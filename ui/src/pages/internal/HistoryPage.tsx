import { For, Show, createMemo, createSignal } from 'solid-js';
import {
  ActionButton,
  EmptyState,
  Favicon,
  LoadingState,
  PageHeader,
  SearchField,
} from './components';
import {
  type StoredRecord,
  formatRecordTime,
  recordTimestamp,
  t,
  useInternalData,
} from './data';

export default function HistoryPage() {
  const { data, locale } = useInternalData();
  const [query, setQuery] = createSignal('');
  const records = () => (data()?.records ?? []) as StoredRecord[];
  const visible = createMemo(() => {
    const needle = query().trim().toLocaleLowerCase();
    return needle
      ? records().filter((record) =>
          `${record.title} ${record.url}`.toLocaleLowerCase().includes(needle),
        )
      : records();
  });
  const day = (value: string) =>
    recordTimestamp(value)?.toDateString() ?? t(locale(), 'earlier');
  return (
    <main class="internal-main">
      <PageHeader
        title={t(locale(), 'history')}
        eyebrow="Fubuki"
        actions={
          <div class="internal-actions compact">
            <ActionButton
              keyName="clearHistoryRange"
              value="lastHour"
              returnUrl="fubuki://history/"
              danger
              confirm={t(locale(), 'confirmAction')}
            >
              {t(locale(), 'clearLastHour')}
            </ActionButton>
            <ActionButton
              keyName="clearHistoryRange"
              value="today"
              returnUrl="fubuki://history/"
              danger
              confirm={t(locale(), 'confirmAction')}
            >
              {t(locale(), 'clearToday')}
            </ActionButton>
            <ActionButton
              keyName="clearHistoryRange"
              value="all"
              returnUrl="fubuki://history/"
              danger
              confirm={t(locale(), 'confirmAction')}
            >
              {t(locale(), 'clearAll')}
            </ActionButton>
          </div>
        }
      />
      <SearchField
        value={query()}
        onInput={setQuery}
        placeholder={t(locale(), 'searchHistory')}
      />
      <Show when={!data.loading} fallback={<LoadingState locale={locale()} />}>
        <Show
          when={!data.error}
          fallback={<LoadingState locale={locale()} error />}
        >
          <Show
            when={visible().length > 0}
            fallback={
              <EmptyState
                icon="◷"
                title={
                  query() ? t(locale(), 'noMatches') : t(locale(), 'noHistory')
                }
              />
            }
          >
            <section class="record-list">
              <For each={visible()}>
                {(record, index) => (
                  <>
                    <Show
                      when={
                        index() === 0 ||
                        day(record.createdAt) !==
                          day(visible()[index() - 1]?.createdAt ?? '')
                      }
                    >
                      <h2 class="record-group">
                        {formatRecordTime(record.createdAt, locale(), true) ||
                          t(locale(), 'earlier')}
                      </h2>
                    </Show>
                    <article class="record-row">
                      <Favicon url={record.faviconUrl} />
                      <a class="record-main" href={record.url}>
                        <strong>{record.title || record.url}</strong>
                        <span>{record.url}</span>
                      </a>
                      <span class="record-time">
                        {formatRecordTime(record.createdAt, locale())}
                      </span>
                      <ActionButton
                        keyName="removeHistory"
                        value={record.url}
                        returnUrl="fubuki://history/"
                        danger
                        confirm={t(locale(), 'confirmAction')}
                      >
                        {t(locale(), 'delete')}
                      </ActionButton>
                    </article>
                  </>
                )}
              </For>
            </section>
          </Show>
        </Show>
      </Show>
    </main>
  );
}
