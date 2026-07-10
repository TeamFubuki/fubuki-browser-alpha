import { createMemo, createSignal, onCleanup, onMount } from 'solid-js';
import { page, tabs } from '../bridge/fubuki';
import { t } from '../i18n';
import {
  activeTab,
  browserState,
  isTabBookmarked,
  toggleBookmark,
} from '../stores/browserStore';
import Omnibox from './Omnibox';

export default function TopBar() {
  const [findOpen, setFindOpen] = createSignal(false);
  const [findText, setFindText] = createSignal('');
  let findInput: HTMLInputElement | undefined;

  const currentTab = createMemo(() => activeTab());
  const isBookmarked = createMemo(() => isTabBookmarked(currentTab()?.url));
  const isLoading = createMemo(() => currentTab()?.isLoading ?? false);
  const canGoBack = createMemo(() => currentTab()?.canGoBack ?? false);
  const canGoForward = createMemo(() => currentTab()?.canGoForward ?? false);

  onMount(() => {
    const showFind = () => {
      setFindOpen(true);
      queueMicrotask(() => {
        findInput?.focus();
        findInput?.select();
      });
    };
    window.addEventListener('fubuki:show-find', showFind);
    onCleanup(() => window.removeEventListener('fubuki:show-find', showFind));
  });

  const submitFind = (forward = true) => {
    const query = findText().trim();
    if (query) void page.find(query, forward);
  };

  const closeFind = () => {
    setFindOpen(false);
    void page.stopFinding();
  };

  const lang = () => browserState.settings.language;

  return (
    <header
      classList={{ 'top-bar': true, private: browserState.isPrivate }}
      aria-label="Navigation"
    >
      <button
        class="topbar-button"
        title={t('common.back', lang())}
        aria-label={t('common.back', lang())}
        disabled={!canGoBack()}
        onClick={() => void tabs.goBack(browserState.activeTabId)}
      >
        <span aria-hidden="true">←</span>
      </button>
      <button
        class="topbar-button"
        title={t('common.forward', lang())}
        aria-label={t('common.forward', lang())}
        disabled={!canGoForward()}
        onClick={() => void tabs.goForward(browserState.activeTabId)}
      >
        <span aria-hidden="true">→</span>
      </button>
      <button
        class="topbar-button"
        title={
          isLoading() ? t('common.stop', lang()) : t('common.reload', lang())
        }
        aria-label={
          isLoading() ? t('common.stop', lang()) : t('common.reload', lang())
        }
        disabled={!currentTab()}
        onClick={() =>
          void (isLoading()
            ? tabs.stop(browserState.activeTabId)
            : tabs.reload(browserState.activeTabId))
        }
      >
        <span aria-hidden="true">{isLoading() ? '×' : '↻'}</span>
      </button>
      <Omnibox />
      <button
        classList={{
          'topbar-button': true,
          bookmarked: isBookmarked(),
        }}
        title={
          isBookmarked()
            ? t('action.removeBookmark', lang())
            : t('action.addBookmark', lang())
        }
        aria-label={
          isBookmarked()
            ? t('action.removeBookmark', lang())
            : t('action.addBookmark', lang())
        }
        disabled={!currentTab()}
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
            ref={(element) => {
              findInput = element;
            }}
            value={findText()}
            placeholder={t('common.find', lang())}
            aria-label={t('common.find', lang())}
            onInput={(event) => {
              const value = event.currentTarget.value;
              setFindText(value);
              if (value.trim()) void page.find(value, true);
              else void page.stopFinding();
            }}
            onKeyDown={(event) => {
              if (event.key === 'Escape') {
                event.preventDefault();
                closeFind();
              }
            }}
            autofocus
          />
          <button
            type="button"
            title={t('find.previous', lang())}
            onClick={() => submitFind(false)}
          >
            ↑
          </button>
          <button
            type="button"
            title={t('find.next', lang())}
            onClick={() => submitFind(true)}
          >
            ↓
          </button>
          <button
            type="button"
            title={t('action.closeFind', lang())}
            onClick={closeFind}
          >
            ×
          </button>
        </form>
      )}
    </header>
  );
}
