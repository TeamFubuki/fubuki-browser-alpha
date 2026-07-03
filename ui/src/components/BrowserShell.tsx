import Sidebar from "./Sidebar";
import TopBar from "./TopBar";
import WebViewArea from "./WebViewArea";

export default function BrowserShell(props: { quietMode: boolean }) {
  return (
    <main classList={{ "browser-shell": true, "quiet-mode": props.quietMode }}>
      <Sidebar />
      <TopBar />
      <WebViewArea />
    </main>
  );
}
