import { createEffect, createSignal } from 'solid-js';
import { tabs } from '../bridge/fubuki';
import { t } from '../i18n';
import { browserState } from '../stores/browserStore';
import { normalizeOmniboxInput } from '../utils/navigation';

function buildSearchUrl(query: string): string {
  const engine = browserState.settings.searchEngine || 'google';
  const custom =
    browserState.settings.customSearchUrl ||
    'https://www.google.com/search?q={query}';
  const encoded = encodeURIComponent(query);
  if (engine === 'duckduckgo') return `https://duckduckgo.com/?q=${encoded}`;
  if (engine === 'bing') return `https://www.bing.com/search?q=${encoded}`;
  if (engine === 'custom') return custom.replace('{query}', encoded);
  return `https://www.google.com/search?q=${encoded}`;
}

export default function Omnibox() {
  const [draft, setDraft] = createSignal('');
  const [focused, setFocused] = createSignal(false);
  let composing = false;
  let lastSyncedTabId = '';
  let lastSyncedUrl = '';

  createEffect(() => {
    const tabId = browserState.activeTabId;
    const tab = browserState.tabs.find((t) => t.id === tabId);
    const url = tab?.url ?? '';

    // Keep typed text while the user is editing, but always accept a URL
    // change reported by the browser (redirects, back/forward, and internal
    // pages) even when the same tab remains active.
    if (
      !focused() ||
      tabId !== lastSyncedTabId ||
      url !== lastSyncedUrl
    ) {
      setDraft(url);
      lastSyncedTabId = tabId;
      lastSyncedUrl = url;
    }
  });

  const submit = () => {
    if (composing) return;
    const raw = draft().trim();
    if (!raw) return;

    const input = normalizeOmniboxInput(raw);
    const url =
      input.kind === 'search' ? buildSearchUrl(input.value) : input.value;

    const tabId = browserState.activeTabId;
    // Navigation must always target an existing tab. A missing active tab is
    // an invalid transient state, not a reason to create a tab implicitly
    // from a search submission.
    if (!tabId) return;
    void tabs.navigate(tabId, url);
  };

  let inputRef: HTMLInputElement | undefined;

  return (
    <form
      class="omnibox"
      onSubmit={(event) => {
        event.preventDefault();
        submit();
      }}
    >
      <input
        ref={(el) => {
          inputRef = el;
        }}
        class="omnibox-input"
        value={draft()}
        placeholder={t(
          'common.searchOrEnterUrl',
          browserState.settings.language,
        )}
        aria-label={t(
          'common.searchOrEnterUrl',
          browserState.settings.language,
        )}
        autocomplete="off"
        autocapitalize="off"
        spellcheck={false}
        onFocus={() => {
          setFocused(true);
          inputRef?.select();
        }}
        onBlur={() => {
          setFocused(false);
          composing = false;
        }}
        onInput={(event) => setDraft(event.currentTarget.value)}
        onCompositionStart={() => {
          composing = true;
        }}
        onCompositionEnd={(event) => {
          composing = false;
          setDraft(event.currentTarget.value);
        }}
        onKeyDown={(event) => {
          if (
            (event.key === 'Enter' || event.key === 'Return') &&
            !composing
          ) {
            event.preventDefault();
            submit();
          }
        }}
      />
    </form>
  );
}
