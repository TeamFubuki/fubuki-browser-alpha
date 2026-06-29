import { fubuki, type Tab } from "../bridge/fubuki";

type Props = {
  tab: Tab;
};

export default function TabItem(props: Props) {
  return (
    <div classList={{ tab: true, active: props.tab.isActive }} title={props.tab.title || props.tab.url}>
      <button class="tab-main" onClick={() => void fubuki.invoke("tabs.activate", { tabId: props.tab.id })}>
        <span classList={{ spinner: props.tab.isLoading, favicon: !props.tab.isLoading }}>
          {!props.tab.isLoading && props.tab.faviconUrl ? <img src={props.tab.faviconUrl} alt="" /> : null}
        </span>
        <span class="tab-title">{props.tab.title || "New Tab"}</span>
      </button>
      <button
        class="tab-close"
        title="Close tab"
        onClick={(event) => {
          event.stopPropagation();
          void fubuki.invoke("tabs.close", { tabId: props.tab.id });
        }}
      >
        <span aria-hidden="true">x</span>
      </button>
    </div>
  );
}
