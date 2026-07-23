import { Logo } from './components';
import { useInternalData } from './data';

export default function NewTabPage() {
  const { locale } = useInternalData();
  return (
    <main class="internal-newtab">
      <div class="newtab-brand">
        <Logo />
        <div>
          <span>Fubuki</span>
          <h1>Browser Alpha</h1>
        </div>
      </div>
      <form class="newtab-search" action="fubuki://newtab/search" method="get">
        <span aria-hidden="true">⌕</span>
        <input
          name="q"
          autofocus
          autocomplete="off"
          aria-label={
            locale() === 'ja'
              ? '検索語句またはURLを入力'
              : 'Search or enter URL'
          }
          placeholder={
            locale() === 'ja'
              ? '検索語句またはURLを入力'
              : 'Search or enter URL'
          }
        />
      </form>
      <nav class="newtab-links" aria-label="Fubuki">
        <a href="fubuki://bookmarks/">
          <span aria-hidden="true">◇</span>
          {locale() === 'ja' ? 'ブックマーク' : 'Bookmarks'}
        </a>
        <a href="fubuki://history/">
          <span aria-hidden="true">◷</span>
          {locale() === 'ja' ? '履歴' : 'History'}
        </a>
        <a href="fubuki://downloads/">
          <span aria-hidden="true">↓</span>
          {locale() === 'ja' ? 'ダウンロード' : 'Downloads'}
        </a>
        <a href="fubuki://settings/">
          <span aria-hidden="true">⚙</span>
          {locale() === 'ja' ? '設定' : 'Settings'}
        </a>
      </nav>
    </main>
  );
}
