import { createResource } from 'solid-js';

export type InternalLocale = 'en' | 'ja';

export type StoredRecord = {
  title: string;
  url: string;
  faviconUrl: string;
  createdAt: string;
};

export type DownloadRecord = {
  url: string;
  path: string;
  state: string;
  percent: number;
  createdAt: string;
};

export type LogRecord = {
  message: string;
  level: string;
  createdAt: string;
};

export type InternalPageData = {
  locale: InternalLocale;
  appearance: 'system' | 'light' | 'dark';
  records?: StoredRecord[] | DownloadRecord[];
  settings?: Record<string, string>;
  profilePath?: string;
  logs?: LogRecord[];
};

const fallback: InternalPageData = { locale: 'en', appearance: 'system' };

function previewData(page: string): InternalPageData {
  const common = { locale: 'en' as const, appearance: 'system' as const };
  if (page === 'bookmarks')
    return {
      ...common,
      records: [
        {
          title: 'SolidJS',
          url: 'https://www.solidjs.com/',
          faviconUrl: '',
          createdAt: '1784156400',
        },
        {
          title: 'Chromium Embedded Framework',
          url: 'https://bitbucket.org/chromiumembedded/cef/',
          faviconUrl: '',
          createdAt: '1784070000',
        },
      ],
    };
  if (page === 'history')
    return {
      ...common,
      records: [
        {
          title: 'Fubuki Browser',
          url: 'https://example.com/fubuki',
          faviconUrl: '',
          createdAt: '1784156400',
        },
        {
          title: 'SolidJS Documentation',
          url: 'https://docs.solidjs.com/',
          faviconUrl: '',
          createdAt: '1784152800',
        },
        {
          title: 'Rust',
          url: 'https://www.rust-lang.org/',
          faviconUrl: '',
          createdAt: '1784070000',
        },
      ],
    };
  if (page === 'downloads')
    return {
      ...common,
      records: [
        {
          url: 'https://example.com/Fubuki.dmg',
          path: '/Users/example/Downloads/Fubuki.dmg',
          state: 'completed',
          percent: 100,
          createdAt: '1784156400',
        },
        {
          url: 'https://example.com/archive.zip',
          path: '/Users/example/Downloads/archive.zip',
          state: 'in_progress',
          percent: 62,
          createdAt: '1784152800',
        },
      ],
    };
  if (page === 'settings')
    return {
      ...common,
      settings: {
        appearance: 'system',
        language: 'system',
        startupBehavior: 'restore',
        homeUrl: 'https://example.com',
        newTabPage: 'blank',
        defaultZoomLevel: '100',
        sidebarVisible: 'show',
        sidebarWidth: '240',
        searchEngine: 'google',
        customSearchUrl: '',
        askBeforeDownload: 'off',
        downloadDirectory: '/Users/example/Downloads',
      },
    };
  if (page === 'debug')
    return {
      ...common,
      profilePath:
        '/Users/example/Library/Application Support/Fubuki Browser Alpha',
      logs: [
        {
          level: 'info',
          message: 'FrostEngine started',
          createdAt: '1784156400',
        },
        {
          level: 'warning',
          message: 'Example diagnostic entry',
          createdAt: '1784152800',
        },
      ],
    };
  return common;
}

async function loadInternalData(): Promise<InternalPageData> {
  const dataUrl = new URL('/data.json', window.location.href);
  const preview = new URLSearchParams(window.location.search).get('page');
  let data: InternalPageData;
  if (
    import.meta.env.DEV &&
    (window.location.hostname === 'localhost' ||
      window.location.hostname === '127.0.0.1') &&
    preview
  ) {
    data = previewData(preview);
  } else {
    const response = await fetch(dataUrl, { cache: 'no-store' });
    if (!response.ok)
      throw new Error(`Internal data request failed (${response.status})`);
    data = (await response.json()) as InternalPageData;
  }
  document.documentElement.lang = data.locale || 'en';
  document.documentElement.dataset.appearance = data.appearance || 'system';
  const titleKey = (preview || window.location.hostname) as InternalLabel;
  if (titleKey in labels.en) document.title = t(data.locale || 'en', titleKey);
  return data;
}

export function useInternalData() {
  const [data] = createResource(loadInternalData);
  return {
    data,
    locale: () => data()?.locale ?? fallback.locale,
  };
}

const labels = {
  ja: {
    bookmarks: 'ブックマーク',
    downloads: 'ダウンロード',
    history: '履歴',
    settings: '設定',
    debug: 'デバッグ',
    searchHistory: '履歴を検索',
    searchSettings: '設定を検索',
    noBookmarks: 'ブックマークはまだありません',
    noDownloads: 'ダウンロードはまだありません',
    noHistory: '履歴はまだありません',
    noMatches: '一致する項目はありません',
    loading: '読み込み中…',
    loadError: 'データを読み込めませんでした',
    delete: '削除',
    remove: '削除',
    open: '開く',
    reveal: 'Finderで表示',
    clearDownloads: 'ダウンロード履歴を消去',
    clearLastHour: '直近1時間を消去',
    clearToday: '今日の履歴を消去',
    clearAll: 'すべて消去',
    confirmAction: 'この操作を実行しますか？',
    completed: '完了',
    failed: '失敗',
    canceled: 'キャンセル済み',
    downloading: 'ダウンロード中',
    starting: '開始中',
    earlier: '以前',
    save: '保存',
    reset: 'リセット',
    noSettings: '一致する設定はありません',
    profilePath: 'プロファイルパス',
    logs: 'ログ',
    noLogs: 'ログはありません',
    actions: '操作',
    openDevTools: 'DevToolsを開く',
  },
  en: {
    bookmarks: 'Bookmarks',
    downloads: 'Downloads',
    history: 'History',
    settings: 'Settings',
    debug: 'Debug',
    searchHistory: 'Search history',
    searchSettings: 'Search settings',
    noBookmarks: 'No bookmarks yet',
    noDownloads: 'No downloads yet',
    noHistory: 'No history yet',
    noMatches: 'No matching items',
    loading: 'Loading…',
    loadError: 'Could not load data',
    delete: 'Delete',
    remove: 'Remove',
    open: 'Open',
    reveal: 'Show in Finder',
    clearDownloads: 'Clear download history',
    clearLastHour: 'Clear last hour',
    clearToday: 'Clear today',
    clearAll: 'Clear all',
    confirmAction: 'Do you want to continue?',
    completed: 'Completed',
    failed: 'Failed',
    canceled: 'Canceled',
    downloading: 'Downloading',
    starting: 'Starting',
    earlier: 'Earlier',
    save: 'Save',
    reset: 'Reset',
    noSettings: 'No matching settings',
    profilePath: 'Profile path',
    logs: 'Logs',
    noLogs: 'No logs',
    actions: 'Actions',
    openDevTools: 'Open DevTools',
  },
} as const;

export type InternalLabel = keyof (typeof labels)['en'];

export function t(locale: InternalLocale, key: InternalLabel): string {
  return labels[locale][key];
}

export function recordTimestamp(value: string): Date | undefined {
  if (!value) return undefined;
  const numeric = Number(value);
  const date = Number.isFinite(numeric)
    ? new Date(numeric < 10_000_000_000 ? numeric * 1000 : numeric)
    : new Date(value);
  return Number.isNaN(date.getTime()) ? undefined : date;
}

export function formatRecordTime(
  value: string,
  locale: InternalLocale,
  dayOnly = false,
): string {
  const date = recordTimestamp(value);
  if (!date) return value;
  return new Intl.DateTimeFormat(
    locale === 'ja' ? 'ja-JP' : 'en-US',
    dayOnly
      ? { year: 'numeric', month: 'short', day: 'numeric' }
      : {
          year: 'numeric',
          month: 'short',
          day: 'numeric',
          hour: '2-digit',
          minute: '2-digit',
        },
  ).format(date);
}
