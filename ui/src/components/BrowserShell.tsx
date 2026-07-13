import {
  browserState,
  clearBrowserError,
  recoverFromBridgeFailure,
  runBrowserAction,
} from '../stores/browserStore';
import Sidebar from './Sidebar';
import TopBar from './TopBar';

export default function BrowserShell(props: { quietMode: boolean }) {
  return (
    <main classList={{ 'browser-shell': true, 'quiet-mode': props.quietMode }}>
      <Sidebar />
      <TopBar />
      <section class="webview-area" aria-hidden="true" />
      {browserState.error && (
        <section class="bridge-error" role="alert" aria-live="assertive">
          <span>
            {browserState.error.method
              ? `${browserState.error.method}: ${browserState.error.message}`
              : browserState.error.message}
          </span>
          <button
            type="button"
            onClick={() => runBrowserAction(recoverFromBridgeFailure())}
          >
            Retry
          </button>
          <button
            type="button"
            onClick={clearBrowserError}
            aria-label="Dismiss error"
          >
            ×
          </button>
        </section>
      )}
    </main>
  );
}
