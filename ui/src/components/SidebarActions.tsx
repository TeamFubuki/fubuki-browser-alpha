import { invokeBridge, tabs } from "../bridge/fubuki";
import { t } from "../i18n";
import { browserState } from "../stores/browserStore";

function activeTabId() {
  return browserState.activeTabId;
}

function openInternal(url: string) {
  const tabId = activeTabId();
  if (tabId) {
    void tabs.navigate(tabId, url);
  } else {
    void invokeBridge("tabs.create", { url, active: true });
  }
}

export default function SidebarActions() {
  return (
    <nav class="sidebar-actions" aria-label="Browser pages">
      <button title={t("common.bookmarks", browserState.settings.language)} aria-label={t("common.bookmarks", browserState.settings.language)} onClick={() => openInternal("fubuki://bookmarks/")}>
        <span aria-hidden="true">★</span>
      </button>
      <button title={t("common.history", browserState.settings.language)} aria-label={t("common.history", browserState.settings.language)} onClick={() => openInternal("fubuki://history/")}>
        <span aria-hidden="true">◷</span>
      </button>
      <button title={t("common.downloads", browserState.settings.language)} aria-label={t("common.downloads", browserState.settings.language)} onClick={() => openInternal("fubuki://downloads/")}>
        <span aria-hidden="true">↓</span>
      </button>
      <button title={t("common.settings", browserState.settings.language)} aria-label={t("common.settings", browserState.settings.language)} onClick={() => openInternal("fubuki://settings/")}>
        <span aria-hidden="true">⚙</span>
      </button>
    </nav>
  );
}
