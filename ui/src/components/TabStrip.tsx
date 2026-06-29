import { For } from "solid-js";
import { fubuki } from "../bridge/fubuki";
import { browserState } from "../stores/browserStore";
import TabItem from "./TabItem";

export default function TabStrip() {
  return (
    <section class="tab-strip" aria-label="Tabs">
      <div class="tabs">
        <For each={browserState.tabs}>{(tab) => <TabItem tab={tab} />}</For>
      </div>
      <button class="icon-button new-tab" title="New tab" onClick={() => void fubuki.invoke("tabs.create", { active: true })}>
        <span aria-hidden="true">+</span>
      </button>
    </section>
  );
}
