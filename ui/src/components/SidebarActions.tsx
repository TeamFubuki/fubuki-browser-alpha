import { t } from "../i18n";
import {
  browserState,
  currentLanguage,
  navigateInternal,
} from "../stores/browserStore";

export default function SidebarActions() {
  const lang = () => currentLanguage();

  return (
    <nav class="sidebar-actions" aria-label="Browser pages">
      <button
        title={t("common.bookmarks", lang())}
        aria-label={t("common.bookmarks", lang())}
        onClick={() => navigateInternal("fubuki://bookmarks/")}
      >
        <span aria-hidden="true">★</span>
      </button>
      <button
        title={t("common.history", lang())}
        aria-label={t("common.history", lang())}
        onClick={() => navigateInternal("fubuki://history/")}
      >
        <span aria-hidden="true">◷</span>
      </button>
      <button
        title={t("common.downloads", lang())}
        aria-label={t("common.downloads", lang())}
        onClick={() => navigateInternal("fubuki://downloads/")}
      >
        <span aria-hidden="true">↓</span>
      </button>
      <button
        title={t("common.settings", lang())}
        aria-label={t("common.settings", lang())}
        onClick={() => navigateInternal("fubuki://settings/")}
      >
        <span aria-hidden="true">⚙</span>
      </button>
    </nav>
  );
}
