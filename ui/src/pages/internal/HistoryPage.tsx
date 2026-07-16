import { createSignal } from 'solid-js';
import { ActionButton, EmptyState, PageHeader } from './components';

export default function HistoryPage() {
  const [query, setQuery] = createSignal('');
  return <main class="internal-main"><PageHeader title="History" /><input class="internal-search" type="search" value={query()} onInput={(event) => setQuery(event.currentTarget.value)} placeholder="Search history" /><div class="internal-actions"><ActionButton keyName="clearHistoryRange" value="lastHour" returnUrl="fubuki://history/" danger>Clear last hour</ActionButton><ActionButton keyName="clearHistoryRange" value="today" returnUrl="fubuki://history/" danger>Clear today</ActionButton><ActionButton keyName="clearHistoryRange" value="all" returnUrl="fubuki://history/" danger>Clear all</ActionButton></div><EmptyState>No history</EmptyState></main>;
}
