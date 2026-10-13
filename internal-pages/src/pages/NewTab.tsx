import { Logo, type Translate } from "../components";
export function NewTab(props: { t: Translate }) {
  return (
    <main class="newtab">
      <Logo />
      <h1>Fubuki Browser Alpha</h1>
      <form action="fubuki://newtab/search" method="get">
        <input
          name="q"
          autofocus
          autocomplete="off"
          aria-label={props.t("Search or enter URL")}
          placeholder={props.t("Search or enter URL")}
        />
      </form>
    </main>
  );
}
