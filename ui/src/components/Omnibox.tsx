import { createEffect, createSignal } from "solid-js";
import { fubuki } from "../bridge/fubuki";
import { browserState } from "../stores/browserStore";

function activeTab() {
  return browserState.tabs.find((tab) => tab.id === browserState.activeTabId);
}

export default function Omnibox() {
  const [draft, setDraft] = createSignal("");

  createEffect(() => {
    setDraft(activeTab()?.url ?? "");
  });

  const submit = () => {
    const tab = activeTab();
    const input = draft().trim();
    if (!tab || !input) return;
    void fubuki.invoke("tabs.navigate", { tabId: tab.id, input });
  };

  return (
    <form
      class="omnibox"
      onSubmit={(event) => {
        event.preventDefault();
        submit();
      }}
    >
      <input
        class="omnibox-input"
        value={draft()}
        aria-label="Search or enter URL"
        autocomplete="off"
        autocapitalize="off"
        spellcheck={false}
        onFocus={(event) => event.currentTarget.select()}
        onInput={(event) => setDraft(event.currentTarget.value)}
      />
    </form>
  );
}
