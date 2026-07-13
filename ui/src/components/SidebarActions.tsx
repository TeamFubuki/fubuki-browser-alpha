import { t } from '../i18n';
import {
  currentLanguage,
  navigateInternal,
  runBrowserAction,
} from '../stores/browserStore';

export default function SidebarActions() {
  const lang = () => currentLanguage();

  return (
    <nav class="sidebar-actions" aria-label="Browser pages">
      <button
        title={t('common.bookmarks', lang())}
        aria-label={t('common.bookmarks', lang())}
        onClick={() =>
          runBrowserAction(navigateInternal('fubuki://bookmarks/'))
        }
      >
        <span aria-hidden="true">★</span>
      </button>
      <button
        title={t('common.history', lang())}
        aria-label={t('common.history', lang())}
        onClick={() => runBrowserAction(navigateInternal('fubuki://history/'))}
      >
        <span aria-hidden="true">◷</span>
      </button>
      <button
        title={t('common.downloads', lang())}
        aria-label={t('common.downloads', lang())}
        onClick={() =>
          runBrowserAction(navigateInternal('fubuki://downloads/'))
        }
      >
        <span aria-hidden="true">↓</span>
      </button>
      <button
        title={t('common.settings', lang())}
        aria-label={t('common.settings', lang())}
        onClick={() => runBrowserAction(navigateInternal('fubuki://settings/'))}
      >
        <span aria-hidden="true">⚙</span>
      </button>
    </nav>
  );
}
