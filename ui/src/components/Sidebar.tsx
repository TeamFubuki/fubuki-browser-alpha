import { fubuki } from "../bridge/fubuki";
import SidebarActions from "./SidebarActions";
import VerticalTabList from "./VerticalTabList";

export default function Sidebar() {
  return (
    <aside class="sidebar" aria-label="Tabs">
      <div class="sidebar-drag-area" />
      <button class="new-tab-button" title="New tab" aria-label="New tab" onClick={() => void fubuki.invoke("tabs.create", { active: true })}>
        <span aria-hidden="true">+</span>
      </button>
      <VerticalTabList />
      <SidebarActions />
    </aside>
  );
}
