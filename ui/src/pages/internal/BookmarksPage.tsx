import { For, Show } from 'solid-js';
import {
  ActionButton,
  EmptyState,
  Favicon,
  LoadingState,
  PageHeader,
} from './components';
import {
  type StoredRecord,
  formatRecordTime,
  t,
  useInternalData,
} from './data';

export default function BookmarksPage() {
  const { data, locale } = useInternalData();
  const records = () => (data()?.records ?? []) as StoredRecord[];
  return (
    <main class="internal-main">
      <PageHeader title={t(locale(), 'bookmarks')} eyebrow="Fubuki" />
      <Show
        when={!data.loading || data() !== undefined}
        fallback={<LoadingState locale={locale()} />}
      >
        <Show
          when={!data.error}
          fallback={<LoadingState locale={locale()} error />}
        >
          <Show
            when={records().length > 0}
            fallback={
              <EmptyState
                icon="◇"
                title={t(locale(), 'noBookmarks')}
                detail={
                  locale() === 'ja'
                    ? 'アドレスバーの星から、よく使うページを保存できます。'
                    : 'Use the star in the address bar to save pages you visit often.'
                }
              />
            }
          >
            <section class="record-list" aria-live="polite">
              <For each={records()}>
                {(record) => (
                  <article class="record-row">
                    <Favicon url={record.faviconUrl} />
                    <a class="record-main" href={record.url} title={record.url}>
                      <strong>{record.title || record.url}</strong>
                      <span>{record.url}</span>
                    </a>
                    <span class="record-time">
                      {formatRecordTime(record.createdAt, locale(), true)}
                    </span>
                    <ActionButton
                      keyName="removeBookmark"
                      value={record.url}
                      danger
                      confirm={t(locale(), 'confirmAction')}
                    >
                      {t(locale(), 'delete')}
                    </ActionButton>
                  </article>
                )}
              </For>
            </section>
          </Show>
        </Show>
      </Show>
    </main>
  );
}
