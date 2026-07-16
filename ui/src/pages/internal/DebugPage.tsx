import { ActionButton, PageHeader } from './components';

export default function DebugPage() {
  return <main class="internal-main"><PageHeader title="Debug" /><section class="debug-grid"><div class="setting-field"><strong>Bridge</strong><span>Internal pages use the restricted Fubuki action surface.</span></div><div class="setting-field"><strong>Actions</strong><div class="internal-actions"><ActionButton keyName="openDevTools" value="1" returnUrl="fubuki://debug/" danger>Open DevTools</ActionButton></div></div></section></main>;
}
