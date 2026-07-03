import { createEffect, createSignal, onCleanup } from "solid-js";
import { tabs } from "../bridge/fubuki";
import { t } from "../i18n";
import { browserState } from "../stores/browserStore";
import { normalizeOmniboxInput } from "../utils/navigation";

export default function Omnibox() {
  const [draft, setDraft] = createSignal("");
  const [focused, setFocused] = createSignal(false);
  const [isComposing, setIsComposing] = createSignal(false);
  let lastSyncedTabId = "";

  createEffect(() => {
    const tabId = browserState.activeTabId;
    const tab = browserState.tabs.find((t) => t.id === tabId);
    const url = tab?.url ?? "";

    if (!focused() || tabId !== lastSyncedTabId) {
      setDraft(url);
      lastSyncedTabId = tabId;
    }
  });

  const submit = () => {
    if (isComposing()) return;
    const tab = browserState.tabs.find((item) => item.id === browserState.activeTabId);
    const input = normalizeOmniboxInput(draft()).value;
    if (!tab || !input) return;
    void tabs.navigate(tab.id, input);
  };

  let inputRef: HTMLInputElement | undefined;

  onCleanup(() => {});

  return (
    <form
      class="omnibox"
      onSubmit={(event) => {
        event.preventDefault();
        if (isComposing()) return;
        submit();
      }}
    >
      <input
        ref={inputRef}
        class="omnibox-input"
        value={draft()}
        placeholder={t("common.searchOrEnterUrl", browserState.settings.language)}
        aria-label={t("common.searchOrEnterUrl", browserState.settings.language)}
        autocomplete="off"
        autocapitalize="off"
        spellcheck={false}
        onFocus={() => {
          setFocused(true);
          inputRef?.select();
        }}
        onBlur={() => setFocused(false)}
        onInput={(event) => setDraft(event.currentTarget.value)}
        onCompositionStart={() => setIsComposing(true)}
        onCompositionEnd={(event) => {
          setIsComposing(false);
          setDraft(event.currentTarget.value);
        }}
        onKeyDown={(event) => {
          if (event.key === "Enter" && (event.isComposing || isComposing())) {
            event.preventDefault();
          }
        }}
      />
    </form>
  );
}
