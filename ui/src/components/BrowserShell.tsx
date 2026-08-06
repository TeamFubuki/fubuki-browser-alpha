import Sidebar from './Sidebar';
import TopBar from './TopBar';
import { browserState } from '../stores/browserStore';

export default function BrowserShell(props: { quietMode: boolean }) {
  return (
    <main
      classList={{ 'browser-shell': true, 'quiet-mode': props.quietMode }}
      data-private={browserState.isPrivate ? 'true' : 'false'}
    >
      <Sidebar />
      <TopBar />
      <section class="webview-area" aria-hidden="true" />
    </main>
  );
}
