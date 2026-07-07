import { createMemo } from 'solid-js';
import SettingsPage from '../internalPages/SettingsPage';
import { activeTab } from '../stores/browserStore';
import Sidebar from './Sidebar';
import TopBar from './TopBar';

export default function BrowserShell(props: { quietMode: boolean }) {
  const isSettingsPage = createMemo(() => {
    const tab = activeTab();
    return tab?.url === 'fubuki://settings' || tab?.url.startsWith('fubuki://settings/');
  });

  return (
    <main classList={{ 'browser-shell': true, 'quiet-mode': props.quietMode }}>
      <Sidebar />
      <TopBar />
      {isSettingsPage() ? (
        <SettingsPage />
      ) : (
        <section class="webview-area" aria-hidden="true" />
      )}
    </main>
  );
}
