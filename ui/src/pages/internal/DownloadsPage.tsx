import { ActionButton, EmptyState, PageHeader } from './components';

export default function DownloadsPage() {
  return <main class="internal-main"><PageHeader title="Downloads" /><div class="internal-actions"><ActionButton keyName="clearData" value="downloads" returnUrl="fubuki://downloads/" danger>Clear downloads</ActionButton></div><EmptyState>No downloads</EmptyState></main>;
}
