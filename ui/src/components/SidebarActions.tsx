import { invokeBridge, tabs } from "../bridge/fubuki";
import { t } from "../i18n";
import { activeTabId, browserState } from "../stores/browserStore";

function openInternal(url: string) {
  const id = activeTabId();
  if (id) {
    void tabs.navigate(id, url);
  } else {
    void invokeBridge("tabs.create", { url, active: true });
  }
}

export default function SidebarActions() {
  const lang = browserState.settings.language;

  return (
    <nav class="sidebar-actions" aria-label="Browser pages">
      <button title={t("common.bookmarks", lang)} aria-label={t("common.bookmarks", lang)} onClick={() => openInternal("fubuki://bookmarks/")}>
        <span aria-hidden="true">★</span>
      </button>
      <button title={t("common.history", lang)} aria-label={t("common.history", lang)} onClick={() => openInternal("fubuki://history/")}>
        <span aria-hidden="true">◷</span>
      </button>
      <button title={t("common.downloads", lang)} aria-label={t("common.downloads", lang)} onClick={() => openInternal("fubuki://downloads/")}>
        <span aria-hidden="true">↓</span>
      </button>
      <button title={t("common.settings", lang)} aria-label={t("common.settings", lang)} onClick={() => openInternal("fubuki://settings/")}>
        <span aria-hidden="true">⚙</span>
      </button>
    </nav>
  );
}
