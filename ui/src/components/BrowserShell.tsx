import Sidebar from './Sidebar';
import TopBar from './TopBar';

export default function BrowserShell(props: { quietMode: boolean }) {
  return (
    <main classList={{ 'browser-shell': true, 'quiet-mode': props.quietMode }}>
      <Sidebar />
      <TopBar />
      <section class="webview-area" aria-hidden="true" />
    </main>
  );
}
