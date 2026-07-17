import { For, Show } from 'solid-js';
import {
  ActionButton,
  EmptyState,
  LoadingState,
  PageHeader,
} from './components';
import { formatRecordTime, t, useInternalData } from './data';

export default function DebugPage() {
  const { data, locale } = useInternalData();
  return (
    <main class="internal-main">
      <PageHeader
        title={t(locale(), 'debug')}
        eyebrow="Fubuki"
        actions={
          <ActionButton keyName="openDevTools" value="1">
            {t(locale(), 'openDevTools')}
          </ActionButton>
        }
      />
      <Show
        when={!data.loading || data() !== undefined}
        fallback={<LoadingState locale={locale()} />}
      >
        <Show
          when={!data.error}
          fallback={<LoadingState locale={locale()} error />}
        >
          <section class="debug-grid">
            <article class="setting-card">
              <header>
                <div>
                  <h2>{t(locale(), 'profilePath')}</h2>
                  <p>FrostEngine · SQLite</p>
                </div>
                <span class="status-pill">v0</span>
              </header>
              <code class="path-value">{data()?.profilePath}</code>
            </article>
            <article class="setting-card">
              <header>
                <div>
                  <h2>{t(locale(), 'logs')}</h2>
                  <p>
                    {locale() === 'ja'
                      ? '最新80件のブラウザログ'
                      : 'The latest 80 browser log entries'}
                  </p>
                </div>
                <span class="status-pill">{data()?.logs?.length ?? 0}</span>
              </header>
              <Show
                when={(data()?.logs?.length ?? 0) > 0}
                fallback={<EmptyState icon="i" title={t(locale(), 'noLogs')} />}
              >
                <div class="log-list">
                  <For each={data()?.logs}>
                    {(log) => (
                      <article class="log-row">
                        <span class={`log-level ${log.level || 'info'}`}>
                          {log.level || 'info'}
                        </span>
                        <div>
                          <strong>{log.message}</strong>
                          <span>
                            {formatRecordTime(log.createdAt, locale())}
                          </span>
                        </div>
                      </article>
                    )}
                  </For>
                </div>
              </Show>
            </article>
          </section>
        </Show>
      </Show>
    </main>
  );
}
