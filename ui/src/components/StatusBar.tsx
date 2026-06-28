import { browserState } from "../stores/browserStore";

type Props = {
  status: string;
};

export default function StatusBar(props: Props) {
  const active = () => browserState.tabs.find((tab) => tab.id === browserState.activeTabId);
  return (
    <footer class="status-bar">
      <span>{active()?.title || "Fubuki Browser Alpha"}</span>
      <span>{props.status}</span>
    </footer>
  );
}
