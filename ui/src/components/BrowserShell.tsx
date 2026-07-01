import Sidebar from "./Sidebar";
import TopBar from "./TopBar";
import WebViewArea from "./WebViewArea";

export default function BrowserShell() {
  return (
    <main class="browser-shell">
      <Sidebar />
      <TopBar />
      <WebViewArea />
    </main>
  );
}
