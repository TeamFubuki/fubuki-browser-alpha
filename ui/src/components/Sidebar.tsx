import { For } from "solid-js";
import { fubuki } from "../bridge/fubuki";
import { browserState, refreshState } from "../stores/browserStore";
import TabItem from "./TabItem";

type Props = {
  onBookmarks: () => void;
  onHistory: () => void;
  onDownloads: () => void;
  onSettings: () => void;
};

export default function Sidebar(props: Props) {
  const label = (en: string, ja: string) => (browserState.settings.language === "ja" ? ja : en);

  const toggleSidebar = () => {
    const next = browserState.settings.sidebarVisible === "collapsed" ? "show" : "collapsed";
    void fubuki.invoke("settings.set", { key: "sidebarVisible", value: next }).then(() => refreshState("settings.saved"));
  };

  return (
    <aside class="sidebar" aria-label={label("Sidebar", "サイドバー")}>
      <div class="sidebar-tabs" aria-label={label("Tabs", "タブ")}>
        <For each={browserState.tabs}>{(tab) => <TabItem tab={tab} />}</For>
      </div>
      <nav class="sidebar-actions" aria-label={label("Browser sections", "ブラウザ項目")}>
        <button class="icon-button" title={label("New tab", "新規タブ")} aria-label={label("New tab", "新規タブ")} onClick={() => void fubuki.invoke("tabs.create", { active: true })}>
          <span aria-hidden="true">+</span>
        </button>
        <button class="icon-button" title={label("Bookmarks", "ブックマーク")} aria-label={label("Bookmarks", "ブックマーク")} onClick={props.onBookmarks}>
          <span aria-hidden="true">★</span>
        </button>
        <button class="icon-button" title={label("History", "履歴")} aria-label={label("History", "履歴")} onClick={props.onHistory}>
          <span aria-hidden="true">◷</span>
        </button>
        <button class="icon-button" title={label("Downloads", "ダウンロード")} aria-label={label("Downloads", "ダウンロード")} onClick={props.onDownloads}>
          <span aria-hidden="true">↓</span>
        </button>
        <button class="icon-button" title={label("Settings", "設定")} aria-label={label("Settings", "設定")} onClick={props.onSettings}>
          <span aria-hidden="true">⚙</span>
        </button>
        <button class="icon-button" title={browserState.settings.sidebarVisible === "collapsed" ? label("Expand sidebar", "サイドバーを展開") : label("Collapse sidebar", "サイドバーを折りたたむ")} aria-label={browserState.settings.sidebarVisible === "collapsed" ? label("Expand sidebar", "サイドバーを展開") : label("Collapse sidebar", "サイドバーを折りたたむ")} onClick={toggleSidebar}>
          <span aria-hidden="true">{browserState.settings.sidebarVisible === "collapsed" ? "›" : "‹"}</span>
        </button>
      </nav>
    </aside>
  );
}
