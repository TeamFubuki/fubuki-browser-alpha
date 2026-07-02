import { fubuki } from "../bridge/fubuki";
import { useSidebarResize } from "../hooks/useSidebarResize";
import SidebarActions from "./SidebarActions";
import VerticalTabList from "./VerticalTabList";

export default function Sidebar() {
  const { resizing, startResize, resetWidth } = useSidebarResize();

  return (
    <aside classList={{ sidebar: true, resizing: resizing() }} aria-label="Tabs">
      <div class="sidebar-drag-area" />
      <button class="new-tab-button" title="New tab" aria-label="New tab" onClick={() => void fubuki.invoke("tabs.create", { active: true })}>
        <span aria-hidden="true">+</span>
      </button>
      <VerticalTabList />
      <SidebarActions />
      <div
        class="sidebar-resize-handle"
        role="separator"
        aria-orientation="vertical"
        aria-label="Resize sidebar"
        title="Resize sidebar"
        onPointerDown={startResize}
        onDblClick={resetWidth}
      />
    </aside>
  );
}
