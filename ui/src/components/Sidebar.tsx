import { tabs } from '../bridge/fubuki';
import { useSidebarResize } from '../hooks/useSidebarResize';
import { t } from '../i18n';
import { MAX_SIDEBAR_WIDTH, MIN_SIDEBAR_WIDTH } from '../sidebarSizing';
import { browserState } from '../stores/browserStore';
import SidebarActions from './SidebarActions';
import VerticalTabList from './VerticalTabList';

export default function Sidebar() {
  const { resizing, width, startResize, resetWidth, resizeWithKeyboard } =
    useSidebarResize();

  const lang = () => browserState.settings.language;

  return (
    <aside
      classList={{ sidebar: true, resizing: resizing() }}
      aria-label={t('common.tabs', lang())}
    >
      <div class="sidebar-drag-area" />
      <button
        class="new-tab-button"
        title={t('common.newTab', lang())}
        aria-label={t('common.newTab', lang())}
        onClick={() => void tabs.create()}
      >
        <span aria-hidden="true">+</span>
      </button>
      <VerticalTabList />
      <SidebarActions />
      <div
        class="sidebar-resize-handle"
        role="separator"
        aria-orientation="vertical"
        aria-valuemin={MIN_SIDEBAR_WIDTH}
        aria-valuemax={MAX_SIDEBAR_WIDTH}
        aria-valuenow={width()}
        aria-label={t('sidebar.resize', lang())}
        title={t('sidebar.resize', lang())}
        onPointerDown={startResize}
        onDblClick={resetWidth}
        onKeyDown={resizeWithKeyboard}
        tabIndex={0}
      />
    </aside>
  );
}
