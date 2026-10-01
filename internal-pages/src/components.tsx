import { For, type JSX } from "solid-js";
import { actionUrl, type Page } from "./data";

export type Translate = (label: string) => string;
export function Action(props: {
  page: Page;
  params: Record<string, string>;
  label: string;
  selected?: boolean;
  danger?: boolean;
  disabled?: boolean;
}) {
  return (
    <form method="post" action={actionUrl(props.params, props.page)}>
      <For each={Object.entries({ ...props.params, return: `fubuki://${props.page}/` })}>
        {([key, value]) => <input type="hidden" name={key} value={value} />}
      </For>
      <button
        class="chip"
        classList={{ selected: props.selected, danger: props.danger }}
        disabled={props.disabled}
      >
        {props.label}
      </button>
    </form>
  );
}
export function Field(props: { id?: string; label: string; children: JSX.Element }) {
  return (
    <section id={props.id} class="field">
      <h2>{props.label}</h2>
      {props.children}
    </section>
  );
}
export function Logo() {
  return <img class="logo" src="./logo.svg" alt="" />;
}
