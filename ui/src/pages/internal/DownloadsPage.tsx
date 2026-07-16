import { For, Show } from 'solid-js';
import {
  ActionButton,
  EmptyState,
  LoadingState,
  PageHeader,
} from './components';
import {
  type DownloadRecord,
  formatRecordTime,
  t,
  useInternalData,
} from './data';

function fileName(record: DownloadRecord) {
  const source = record.path || record.url;
  return source.split(/[\\/]/).pop() || source;
}

export default function DownloadsPage() {
  const { data, locale } = useInternalData();
  const records = () => (data()?.records ?? []) as DownloadRecord[];
  const status = (record: DownloadRecord) => {
    if (record.state === 'completed') return t(locale(), 'completed');
    if (record.state === 'failed') return t(locale(), 'failed');
    if (record.state === 'canceled') return t(locale(), 'canceled');
    return `${t(locale(), record.state === 'started' ? 'starting' : 'downloading')} ${Math.max(0, Math.min(100, record.percent))}%`;
  };
  return (
    <main class="internal-main">
      <PageHeader
        title={t(locale(), 'downloads')}
        eyebrow="Fubuki"
        actions={
          <ActionButton
            keyName="clearData"
            value="downloads"
            returnUrl="fubuki://downloads/"
            danger
            confirm={t(locale(), 'confirmAction')}
          >
            {t(locale(), 'clearDownloads')}
          </ActionButton>
        }
      />
      <Show when={!data.loading} fallback={<LoadingState locale={locale()} />}>
        <Show
          when={!data.error}
          fallback={<LoadingState locale={locale()} error />}
        >
          <Show
            when={records().length > 0}
            fallback={
              <EmptyState
                icon="↓"
                title={t(locale(), 'noDownloads')}
                detail={
                  locale() === 'ja'
                    ? 'ダウンロードしたファイルはここに表示されます。'
                    : 'Downloaded files will appear here.'
                }
              />
            }
          >
            <section class="record-list">
              <For each={records()}>
                {(record) => {
                  const value = record.path || record.url;
                  const done = record.state === 'completed';
                  return (
                    <article class="download-row">
                      <span
                        class={`download-icon ${record.state}`}
                        aria-hidden="true"
                      >
                        ↓
                      </span>
                      <div class="record-main">
                        <strong>{fileName(record)}</strong>
                        <span>{record.path || record.url}</span>
                        <div class="progress-line">
                          <span>{status(record)}</span>
                          <div class="progress-track">
                            <i
                              style={{
                                width: `${done ? 100 : Math.max(0, Math.min(100, record.percent))}%`,
                              }}
                            />
                          </div>
                        </div>
                      </div>
                      <span class="record-time">
                        {formatRecordTime(record.createdAt, locale())}
                      </span>
                      <div class="internal-actions compact">
                        <ActionButton
                          keyName="openDownload"
                          value={record.path}
                          returnUrl="fubuki://downloads/"
                          post
                          disabled={!record.path}
                        >
                          {t(locale(), 'open')}
                        </ActionButton>
                        <ActionButton
                          keyName="revealDownload"
                          value={record.path}
                          returnUrl="fubuki://downloads/"
                          post
                          disabled={!record.path}
                        >
                          {t(locale(), 'reveal')}
                        </ActionButton>
                        <ActionButton
                          keyName="removeDownload"
                          value={value}
                          returnUrl="fubuki://downloads/"
                          danger
                          confirm={t(locale(), 'confirmAction')}
                        >
                          {t(locale(), 'remove')}
                        </ActionButton>
                      </div>
                    </article>
                  );
                }}
              </For>
            </section>
          </Show>
        </Show>
      </Show>
    </main>
  );
}
