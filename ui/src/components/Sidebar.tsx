import { invokeBridge } from '../bridge/fubuki';
import { useSidebarResize } from '../hooks/useSidebarResize';
import { t } from '../i18n';
import { browserState } from '../stores/browserStore';
import SidebarActions from './SidebarActions';
import VerticalTabList from './VerticalTabList';

export default function Sidebar() {
  const { resizing, startResize, resetWidth } = useSidebarResize();

  return (
    <aside
      classList={{ sidebar: true, resizing: resizing() }}
      aria-label={t('common.tabs', browserState.settings.language)}
    >
      <div class="sidebar-drag-area" />
      <button
        class="new-tab-button"
        title={t('common.newTab', browserState.settings.language)}
        aria-label={t('common.newTab', browserState.settings.language)}
        onClick={() => void invokeBridge('tabs.create', { active: true })}
      >
        <span aria-hidden="true">+</span>
      </button>
      <VerticalTabList />
      <SidebarActions />
      <div
        class="sidebar-resize-handle"
        role="separator"
        aria-orientation="vertical"
        aria-label={t('sidebar.resize', browserState.settings.language)}
        title={t('sidebar.resize', browserState.settings.language)}
        onPointerDown={startResize}
        onDblClick={resetWidth}
      />
    </aside>
  );
}
