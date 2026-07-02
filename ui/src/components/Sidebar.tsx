import { createSignal } from "solid-js";
import { fubuki } from "../bridge/fubuki";
import { browserState, refreshState } from "../stores/browserStore";
import SidebarActions from "./SidebarActions";
import VerticalTabList from "./VerticalTabList";

const DEFAULT_SIDEBAR_WIDTH = 196;
const MIN_SIDEBAR_WIDTH = 168;
const MAX_SIDEBAR_WIDTH = 280;

function clampSidebarWidth(value: number) {
  return Math.min(MAX_SIDEBAR_WIDTH, Math.max(MIN_SIDEBAR_WIDTH, Math.round(value)));
}

export default function Sidebar() {
  const [resizing, setResizing] = createSignal(false);
  let startX = 0;
  let startWidth = DEFAULT_SIDEBAR_WIDTH;
  let pendingWidth = DEFAULT_SIDEBAR_WIDTH;
  let animationFrame = 0;

  const saveWidth = (width: number) =>
    fubuki.invoke("settings.set", { key: "sidebarWidth", value: String(clampSidebarWidth(width)) }).then(() => refreshState("setting.changed"));

  const applyLiveWidth = () => {
    animationFrame = 0;
    document.documentElement.style.setProperty("--sidebar-width", `${pendingWidth}px`);
    void fubuki.invoke("ui.setSidebarWidth", { width: pendingWidth });
  };

  const startResize = (event: PointerEvent) => {
    event.preventDefault();
    startX = event.clientX;
    startWidth = clampSidebarWidth(Number(browserState.settings.sidebarWidth) || DEFAULT_SIDEBAR_WIDTH);
    pendingWidth = startWidth;
    document.documentElement.dataset.sidebarResizing = "true";
    setResizing(true);

    const onPointerMove = (moveEvent: PointerEvent) => {
      pendingWidth = clampSidebarWidth(startWidth + moveEvent.clientX - startX);
      if (!animationFrame) {
        animationFrame = requestAnimationFrame(applyLiveWidth);
      }
    };

    const onPointerUp = (upEvent: PointerEvent) => {
      const width = clampSidebarWidth(startWidth + upEvent.clientX - startX);
      if (animationFrame) {
        cancelAnimationFrame(animationFrame);
        animationFrame = 0;
      }
      document.documentElement.style.setProperty("--sidebar-width", `${width}px`);
      document.removeEventListener("pointermove", onPointerMove);
      document.removeEventListener("pointerup", onPointerUp);
      delete document.documentElement.dataset.sidebarResizing;
      setResizing(false);
      void saveWidth(width);
    };

    document.addEventListener("pointermove", onPointerMove);
    document.addEventListener("pointerup", onPointerUp, { once: true });
  };

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
        onDblClick={() => {
          document.documentElement.style.setProperty("--sidebar-width", `${DEFAULT_SIDEBAR_WIDTH}px`);
          void saveWidth(DEFAULT_SIDEBAR_WIDTH);
        }}
      />
    </aside>
  );
}
