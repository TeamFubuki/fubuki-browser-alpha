import { For, Show, createMemo, createSignal } from 'solid-js';
import {
  ActionButton,
  EmptyState,
  LoadingState,
  PageHeader,
  SearchField,
  SettingChoice,
  SettingInput,
} from './components';
import type { InternalLocale } from './data';
import { t, useInternalData } from './data';

const copy = {
  en: {
    general: [
      'General',
      'Choose what opens when Fubuki starts and where Home leads.',
    ],
    appearance: [
      'Appearance',
      'Match macOS or choose a light or dark interface.',
    ],
    language: [
      'Language',
      'Choose the language used by browser and internal pages.',
    ],
    tabs: ['Tabs', 'Choose the new tab destination and default page zoom.'],
    windows: ['Window', 'Control sidebar visibility and its default width.'],
    search: ['Search', 'Choose the search engine used by the address bar.'],
    privacy: ['Privacy', 'Remove local browsing records and website data.'],
    downloads: [
      'Downloads',
      'Choose download prompts and the default destination.',
    ],
    developer: ['Developer', 'Open diagnostics and inspect the browser shell.'],
    startup: 'On startup',
    newTab: 'New tab',
    restore: 'Restore previous session',
    home: 'Home page',
    homeUrl: 'Home page URL',
    system: 'System',
    light: 'Light',
    dark: 'Dark',
    japanese: 'Japanese',
    english: 'English',
    newTabDestination: 'New tab destination',
    blank: 'Blank new tab',
    homeOnNewTab: 'Home page',
    zoom: 'Default zoom (%)',
    sidebar: 'Sidebar',
    show: 'Show',
    hide: 'Hide',
    sidebarWidth: 'Sidebar width (px)',
    google: 'Google',
    duckduckgo: 'DuckDuckGo',
    bing: 'Bing',
    custom: 'Custom',
    customSearch: 'Custom search URL',
    history: 'History',
    cookies: 'Cookies',
    cache: 'Cache',
    allData: 'All browsing data',
    ask: 'Ask where to save each file',
    automatic: 'Download automatically',
    directory: 'Download directory',
    debugPage: 'Open debug page',
    resetValue: 'Reset this setting',
  },
  ja: {
    general: ['一般', 'Fubukiの起動時に開く内容とホームの移動先を設定します。'],
    appearance: [
      '外観',
      'macOSに合わせるか、ライトまたはダーク表示を選びます。',
    ],
    language: ['言語', 'ブラウザと内部ページで使用する言語を選びます。'],
    tabs: ['タブ', '新しいタブの移動先と既定の表示倍率を設定します。'],
    windows: ['ウィンドウ', 'サイドバーの表示と既定の幅を設定します。'],
    search: ['検索', 'アドレスバーで使用する検索エンジンを選びます。'],
    privacy: ['プライバシー', '閲覧記録やWebサイトデータを削除します。'],
    downloads: ['ダウンロード', '保存時の確認と既定の保存先を設定します。'],
    developer: ['開発', '診断情報を開き、ブラウザシェルを検証します。'],
    startup: '起動時',
    newTab: '新しいタブ',
    restore: '前回のセッションを復元',
    home: 'ホームページ',
    homeUrl: 'ホームページURL',
    system: 'システム設定',
    light: 'ライト',
    dark: 'ダーク',
    japanese: '日本語',
    english: '英語',
    newTabDestination: '新しいタブの表示',
    blank: '空の新規タブ',
    homeOnNewTab: 'ホームページ',
    zoom: '既定の表示倍率（%）',
    sidebar: 'サイドバー',
    show: '表示',
    hide: '非表示',
    sidebarWidth: 'サイドバー幅（px）',
    google: 'Google',
    duckduckgo: 'DuckDuckGo',
    bing: 'Bing',
    custom: 'カスタム',
    customSearch: 'カスタム検索URL',
    history: '履歴',
    cookies: 'Cookie',
    cache: 'キャッシュ',
    allData: 'すべての閲覧データ',
    ask: 'ファイルごとに保存先を確認',
    automatic: '自動でダウンロード',
    directory: 'ダウンロード先',
    debugPage: 'デバッグページを開く',
    resetValue: 'この設定をリセット',
  },
} as const;

const sectionIds = [
  'general',
  'appearance',
  'language',
  'tabs',
  'windows',
  'search',
  'privacy',
  'downloads',
  'developer',
] as const;
type SectionId = (typeof sectionIds)[number];
type CopyKey = Exclude<keyof (typeof copy)['en'], SectionId>;

function Section(props: {
  id: SectionId;
  locale: InternalLocale;
  settings: Record<string, string>;
}) {
  const c = (key: CopyKey) => copy[props.locale][key];
  const selected = (key: string, fallback: string) =>
    props.settings[key] || fallback;
  const reset = (key: string) => (
    <ActionButton keyName="resetSetting" value={key}>
      {c('resetValue')}
    </ActionButton>
  );
  return (
    <section id={props.id} class="setting-card" data-setting-section>
      <header>
        <div>
          <h2>{copy[props.locale][props.id][0]}</h2>
          <p>{copy[props.locale][props.id][1]}</p>
        </div>
      </header>
      <Show when={props.id === 'general'}>
        <div class="setting-control">
          <span>{c('startup')}</span>
          <div class="internal-actions">
            <SettingChoice
              keyName="startupBehavior"
              value="newTab"
              label={c('newTab')}
              selected={selected('startupBehavior', 'newTab') === 'newTab'}
            />
            <SettingChoice
              keyName="startupBehavior"
              value="restore"
              label={c('restore')}
              selected={selected('startupBehavior', 'newTab') === 'restore'}
            />
            <SettingChoice
              keyName="startupBehavior"
              value="homePage"
              label={c('home')}
              selected={selected('startupBehavior', 'newTab') === 'homePage'}
            />
          </div>
        </div>
        <SettingInput
          keyName="homeUrl"
          label={c('homeUrl')}
          value={selected('homeUrl', '')}
          placeholder="https://example.com"
          locale={props.locale}
        />
        {reset('startupBehavior')}
      </Show>
      <Show when={props.id === 'appearance'}>
        <div class="internal-actions">
          <SettingChoice
            keyName="appearance"
            value="system"
            label={c('system')}
            selected={selected('appearance', 'system') === 'system'}
          />
          <SettingChoice
            keyName="appearance"
            value="light"
            label={c('light')}
            selected={selected('appearance', 'system') === 'light'}
          />
          <SettingChoice
            keyName="appearance"
            value="dark"
            label={c('dark')}
            selected={selected('appearance', 'system') === 'dark'}
          />
        </div>
        {reset('appearance')}
      </Show>
      <Show when={props.id === 'language'}>
        <div class="internal-actions">
          <SettingChoice
            keyName="language"
            value="system"
            label={c('system')}
            selected={selected('language', 'system') === 'system'}
          />
          <SettingChoice
            keyName="language"
            value="ja"
            label={c('japanese')}
            selected={selected('language', 'system') === 'ja'}
          />
          <SettingChoice
            keyName="language"
            value="en"
            label={c('english')}
            selected={selected('language', 'system') === 'en'}
          />
        </div>
        {reset('language')}
      </Show>
      <Show when={props.id === 'tabs'}>
        <div class="setting-control">
          <span>{c('newTabDestination')}</span>
          <div class="internal-actions">
            <SettingChoice
              keyName="newTabPage"
              value="blank"
              label={c('blank')}
              selected={selected('newTabPage', 'blank') === 'blank'}
            />
            <SettingChoice
              keyName="newTabPage"
              value="home"
              label={c('homeOnNewTab')}
              selected={selected('newTabPage', 'blank') === 'home'}
            />
          </div>
        </div>
        <SettingInput
          keyName="defaultZoomLevel"
          label={c('zoom')}
          value={selected('defaultZoomLevel', '100')}
          locale={props.locale}
          type="number"
          min="25"
          max="500"
        />
        {reset('defaultZoomLevel')}
      </Show>
      <Show when={props.id === 'windows'}>
        <div class="setting-control">
          <span>{c('sidebar')}</span>
          <div class="internal-actions">
            <SettingChoice
              keyName="sidebarVisible"
              value="show"
              label={c('show')}
              selected={selected('sidebarVisible', 'show') === 'show'}
            />
            <SettingChoice
              keyName="sidebarVisible"
              value="hide"
              label={c('hide')}
              selected={selected('sidebarVisible', 'show') === 'hide'}
            />
          </div>
        </div>
        <SettingInput
          keyName="sidebarWidth"
          label={c('sidebarWidth')}
          value={selected('sidebarWidth', '240')}
          locale={props.locale}
          type="number"
          min="180"
          max="480"
        />
        {reset('sidebarWidth')}
      </Show>
      <Show when={props.id === 'search'}>
        <div class="internal-actions">
          <For each={['google', 'duckduckgo', 'bing', 'custom'] as const}>
            {(engine) => (
              <SettingChoice
                keyName="searchEngine"
                value={engine}
                label={c(engine)}
                selected={selected('searchEngine', 'google') === engine}
              />
            )}
          </For>
        </div>
        <SettingInput
          keyName="customSearchUrl"
          label={c('customSearch')}
          value={selected('customSearchUrl', '')}
          placeholder="https://example.com/search?q={query}"
          locale={props.locale}
        />
        {reset('searchEngine')}
      </Show>
      <Show when={props.id === 'privacy'}>
        <div class="internal-actions">
          <ActionButton
            keyName="clearData"
            value="history"
            danger
            confirm={t(props.locale, 'confirmAction')}
          >
            {c('history')}
          </ActionButton>
          <ActionButton
            keyName="clearData"
            value="cookies"
            danger
            confirm={t(props.locale, 'confirmAction')}
          >
            {c('cookies')}
          </ActionButton>
          <ActionButton
            keyName="clearData"
            value="cache"
            danger
            confirm={t(props.locale, 'confirmAction')}
          >
            {c('cache')}
          </ActionButton>
          <ActionButton
            keyName="clearData"
            value="all"
            danger
            confirm={t(props.locale, 'confirmAction')}
          >
            {c('allData')}
          </ActionButton>
        </div>
      </Show>
      <Show when={props.id === 'downloads'}>
        <div class="internal-actions">
          <SettingChoice
            keyName="askBeforeDownload"
            value="on"
            label={c('ask')}
            selected={selected('askBeforeDownload', 'off') === 'on'}
          />
          <SettingChoice
            keyName="askBeforeDownload"
            value="off"
            label={c('automatic')}
            selected={selected('askBeforeDownload', 'off') === 'off'}
          />
        </div>
        <SettingInput
          keyName="downloadDirectory"
          label={c('directory')}
          value={selected('downloadDirectory', '')}
          locale={props.locale}
        />
        {reset('downloadDirectory')}
      </Show>
      <Show when={props.id === 'developer'}>
        <a class="internal-button" href="fubuki://debug/">
          {c('debugPage')}
        </a>
      </Show>
    </section>
  );
}

export default function SettingsPage() {
  const { data, locale } = useInternalData();
  const [filter, setFilter] = createSignal('');
  const visible = createMemo(() => {
    const needle = filter().trim().toLocaleLowerCase();
    return sectionIds.filter(
      (id) =>
        !needle ||
        copy[locale()][id].join(' ').toLocaleLowerCase().includes(needle),
    );
  });
  return (
    <main class="internal-main">
      <PageHeader title={t(locale(), 'settings')} eyebrow="Fubuki" />
      <Show
        when={!data.loading || data() !== undefined}
        fallback={<LoadingState locale={locale()} />}
      >
        <Show
          when={!data.error}
          fallback={<LoadingState locale={locale()} error />}
        >
          <div class="settings-layout">
            <nav class="settings-nav" aria-label={t(locale(), 'settings')}>
              <For each={sectionIds}>
                {(id) => <a href={`#${id}`}>{copy[locale()][id][0]}</a>}
              </For>
            </nav>
            <div class="settings-content">
              <SearchField
                value={filter()}
                onInput={setFilter}
                placeholder={t(locale(), 'searchSettings')}
              />
              <Show
                when={visible().length > 0}
                fallback={
                  <EmptyState icon="⌕" title={t(locale(), 'noSettings')} />
                }
              >
                <For each={visible()}>
                  {(id) => (
                    <Section
                      id={id}
                      locale={locale()}
                      settings={data()?.settings ?? {}}
                    />
                  )}
                </For>
              </Show>
            </div>
          </div>
        </Show>
      </Show>
    </main>
  );
}
