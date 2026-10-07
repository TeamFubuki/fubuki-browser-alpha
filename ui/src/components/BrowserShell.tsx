import { createMemo, Show } from 'solid-js';
import { tabs } from '../bridge/fubuki';
import { t } from '../i18n';
import Sidebar from './Sidebar';
import TopBar from './TopBar';
import { activeTab, browserState } from '../stores/browserStore';

export default function BrowserShell(props: { quietMode: boolean }) {
  const rendererTab = createMemo(() => {
    const tab = activeTab();
    return tab && tab.rendererStatus !== 'healthy' ? tab : undefined;
  });
  const lang = () => browserState.settings.language;

  return (
    <main
      classList={{ 'browser-shell': true, 'quiet-mode': props.quietMode }}
      data-private={browserState.isPrivate ? 'true' : 'false'}
    >
      <Sidebar />
      <TopBar />
      <section class="webview-area">
        <Show when={rendererTab()}>
          {(tab) => (
            <aside
              class="renderer-recovery-card"
              aria-live="assertive"
              aria-atomic="true"
              aria-labelledby="renderer-recovery-title"
            >
              <span class="renderer-recovery-mark" aria-hidden="true">
                {tab().rendererStatus === 'recovering' ? '↻' : '!'}
              </span>
              <h1 id="renderer-recovery-title">
                {tab().rendererStatus === 'unresponsive'
                  ? t('renderer.unresponsiveTitle', lang())
                  : tab().rendererStatus === 'recovering'
                    ? t('renderer.recoveringTitle', lang())
                    : t('renderer.crashedTitle', lang())}
              </h1>
              <p>
                {tab().rendererStatus === 'unresponsive'
                  ? t('renderer.unresponsiveDescription', lang())
                  : tab().rendererStatus === 'recovering'
                    ? t('renderer.recoveringDescription', lang())
                    : t('renderer.crashedDescription', lang())}
              </p>
              <Show when={tab().rendererDiagnostic || tab().rendererErrorCode}>
                <details class="renderer-recovery-diagnostics">
                  <summary>{t('renderer.diagnostics', lang())}</summary>
                  <Show when={tab().rendererErrorCode !== 0}>
                    <p>
                      {t('renderer.errorCode', lang())}:{' '}
                      {tab().rendererErrorCode}
                    </p>
                  </Show>
                  <Show when={tab().rendererDiagnostic}>
                    <p>{tab().rendererDiagnostic}</p>
                  </Show>
                </details>
              </Show>
              <div class="renderer-recovery-actions">
                <Show when={tab().rendererStatus === 'unresponsive'}>
                  <button
                    type="button"
                    onClick={() => void tabs.waitForRenderer(tab().id)}
                  >
                    {t('renderer.wait', lang())}
                  </button>
                </Show>
                <Show when={tab().rendererStatus !== 'recovering'}>
                  <button
                    type="button"
                    class="primary"
                    onClick={() => void tabs.reload(tab().id)}
                  >
                    {t('renderer.reload', lang())}
                  </button>
                </Show>
              </div>
            </aside>
          )}
        </Show>
      </section>
    </main>
  );
}
