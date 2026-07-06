import { createSignal, onCleanup, onMount } from 'solid-js';
import { fubuki, page } from '../bridge/fubuki';
import { browserState, refreshState } from '../stores/browserStore';
import Omnibox from './Omnibox';

function activeTab() {
  return browserState.tabs.find((tab) => tab.id === browserState.activeTabId);
}

function isBookmarked() {
  const tab = activeTab();
  return (
    !!tab?.url &&
    browserState.bookmarks.some((bookmark) => bookmark.url === tab.url)
  );
}

async function toggleBookmark() {
  const tab = activeTab();
  if (
    !tab?.url ||
    tab.url.startsWith('fubuki://') ||
    tab.url.startsWith('data:')
  )
    return;
  if (isBookmarked()) {
    await fubuki.invoke('bookmarks.remove', { url: tab.url });
  } else {
    await fubuki.invoke('bookmarks.save', {
      title: tab.title || tab.url,
      url: tab.url,
      faviconUrl: tab.faviconUrl || '',
    });
  }
  await refreshState('bookmarks.changed');
}

export default function TopBar() {
  const [findOpen, setFindOpen] = createSignal(false);
  const [findText, setFindText] = createSignal('');

  onMount(() => {
    const showFind = () => setFindOpen(true);
    window.addEventListener('fubuki:show-find', showFind);
    onCleanup(() => window.removeEventListener('fubuki:show-find', showFind));
  });

  const submitFind = (forward = true) => {
    const query = findText().trim();
    if (query) void page.find(query, forward);
  };

  return (
    <header
      classList={{ 'top-bar': true, private: browserState.isPrivate }}
      aria-label="Navigation"
    >
      <button
        class="topbar-button"
        title="Back"
        aria-label="Back"
        disabled={!activeTab()?.canGoBack}
        onClick={() =>
          void fubuki.invoke('tabs.goBack', { tabId: browserState.activeTabId })
        }
      >
        <span aria-hidden="true">←</span>
      </button>
      <button
        class="topbar-button"
        title="Forward"
        aria-label="Forward"
        disabled={!activeTab()?.canGoForward}
        onClick={() =>
          void fubuki.invoke('tabs.goForward', {
            tabId: browserState.activeTabId,
          })
        }
      >
        <span aria-hidden="true">→</span>
      </button>
      <button
        class="topbar-button"
        title={activeTab()?.isLoading ? 'Stop' : 'Reload'}
        aria-label={activeTab()?.isLoading ? 'Stop' : 'Reload'}
        disabled={!activeTab()}
        onClick={() =>
          void fubuki.invoke(
            activeTab()?.isLoading ? 'tabs.stop' : 'tabs.reload',
            { tabId: browserState.activeTabId },
          )
        }
      >
        <span aria-hidden="true">{activeTab()?.isLoading ? '×' : '↻'}</span>
      </button>
      <Omnibox />
      <button
        classList={{ 'topbar-button': true, bookmarked: isBookmarked() }}
        title={isBookmarked() ? 'Remove bookmark' : 'Add bookmark'}
        aria-label={isBookmarked() ? 'Remove bookmark' : 'Add bookmark'}
        disabled={
          !activeTab()?.url ||
          activeTab()?.url.startsWith('fubuki://') ||
          activeTab()?.url.startsWith('data:')
        }
        onClick={() => void toggleBookmark()}
      >
        <span aria-hidden="true">{isBookmarked() ? '★' : '☆'}</span>
      </button>
      {findOpen() && (
        <form
          class="find-bar"
          onSubmit={(event) => {
            event.preventDefault();
            submitFind(true);
          }}
        >
          <input
            value={findText()}
            placeholder="Find"
            aria-label="Find in page"
            onInput={(event) => setFindText(event.currentTarget.value)}
            autofocus
          />
          <button
            type="button"
            title="Previous match"
            onClick={() => submitFind(false)}
          >
            ↑
          </button>
          <button
            type="button"
            title="Next match"
            onClick={() => submitFind(true)}
          >
            ↓
          </button>
          <button
            type="button"
            title="Close find"
            onClick={() => {
              setFindOpen(false);
              void page.stopFinding();
            }}
          >
            ×
          </button>
        </form>
      )}
    </header>
  );
}
