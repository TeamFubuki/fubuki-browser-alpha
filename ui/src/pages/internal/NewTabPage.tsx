import { Logo } from './components';

export default function NewTabPage() {
  return <main class="internal-newtab"><Logo /><h1>Fubuki Browser Alpha</h1><form action="fubuki://newtab/search" method="get"><input name="q" autofocus autocomplete="off" placeholder="Search or enter URL" /></form></main>;
}
