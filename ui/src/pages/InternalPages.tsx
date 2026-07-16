import BookmarksPage from './internal/BookmarksPage';
import type { Component } from 'solid-js';
import DebugPage from './internal/DebugPage';
import DownloadsPage from './internal/DownloadsPage';
import HistoryPage from './internal/HistoryPage';
import NewTabPage from './internal/NewTabPage';
import SettingsPage from './internal/SettingsPage';

type Page =
  | 'newtab'
  | 'bookmarks'
  | 'downloads'
  | 'history'
  | 'settings'
  | 'debug';

const pages = {
  newtab: { title: 'New Tab', component: NewTabPage },
  bookmarks: { title: 'Bookmarks', component: BookmarksPage },
  downloads: { title: 'Downloads', component: DownloadsPage },
  history: { title: 'History', component: HistoryPage },
  settings: { title: 'Settings', component: SettingsPage },
  debug: { title: 'Debug', component: DebugPage },
} satisfies Record<Page, { title: string; component: Component }>;

function pageFromHost(): Page {
  const host = window.location.hostname;
  const preview = new URLSearchParams(window.location.search).get('page');
  if (
    (host === 'localhost' || host === '127.0.0.1') &&
    preview &&
    preview in pages
  ) {
    return preview as Page;
  }
  return host in pages ? (host as Page) : 'newtab';
}

export default function InternalPages() {
  const page = pages[pageFromHost()];
  document.title = page.title;
  document.documentElement.dataset.page = pageFromHost();
  const PageComponent = page.component;
  return <PageComponent />;
}
