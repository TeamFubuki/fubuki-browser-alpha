import { fubuki } from '../bridge/fubuki';
import { browserState } from '../stores/browserStore';

function activeTabId() {
  return browserState.activeTabId;
}

function openInternal(url: string) {
  const tabId = activeTabId();
  if (tabId) {
    void fubuki.invoke('tabs.navigate', { tabId, input: url });
  } else {
    void fubuki.invoke('tabs.create', { url, active: true });
  }
}

export default function SidebarActions() {
  return (
    <nav class="sidebar-actions" aria-label="Browser pages">
      <button
        title="Bookmarks"
        aria-label="Bookmarks"
        onClick={() => openInternal('fubuki://bookmarks/')}
      >
        <span aria-hidden="true">★</span>
      </button>
      <button
        title="History"
        aria-label="History"
        onClick={() => openInternal('fubuki://history/')}
      >
        <span aria-hidden="true">◷</span>
      </button>
      <button
        title="Downloads"
        aria-label="Downloads"
        onClick={() => openInternal('fubuki://downloads/')}
      >
        <span aria-hidden="true">↓</span>
      </button>
      <button
        title="Settings"
        aria-label="Settings"
        onClick={() => openInternal('fubuki://settings/')}
      >
        <span aria-hidden="true">⚙</span>
      </button>
    </nav>
  );
}
