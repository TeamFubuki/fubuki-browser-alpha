import { createEffect, createSignal, onCleanup } from 'solid-js';
import { fubuki } from '../bridge/fubuki';
import { browserState } from '../stores/browserStore';

function activeTab() {
  return browserState.tabs.find((tab) => tab.id === browserState.activeTabId);
}

function isNonNavigableUrl(url: string | undefined): boolean {
  return !url || url.startsWith('fubuki://') || url.startsWith('data:');
}

export default function Omnibox() {
  const [draft, setDraft] = createSignal('');
  const [focused, setFocused] = createSignal(false);
  let lastSyncedTabId = '';

  createEffect(() => {
    const tabId = browserState.activeTabId;
    const tab = browserState.tabs.find((t) => t.id === tabId);
    const url = tab?.url ?? '';

    if (!focused() || tabId !== lastSyncedTabId) {
      setDraft(url);
      lastSyncedTabId = tabId;
    }
  });

  const submit = () => {
    const tab = activeTab();
    const input = draft().trim();
    if (!tab || !input) return;
    void fubuki.invoke('tabs.navigate', { tabId: tab.id, input });
  };

  let inputRef: HTMLInputElement | undefined;

  onCleanup(() => {});

  return (
    <form
      class="omnibox"
      onSubmit={(event) => {
        event.preventDefault();
        submit();
      }}
    >
      <input
        ref={inputRef}
        class="omnibox-input"
        value={draft()}
        aria-label="Search or enter URL"
        autocomplete="off"
        autocapitalize="off"
        spellcheck={false}
        onFocus={() => {
          setFocused(true);
          inputRef?.select();
        }}
        onBlur={() => setFocused(false)}
        onInput={(event) => setDraft(event.currentTarget.value)}
      />
    </form>
  );
}
