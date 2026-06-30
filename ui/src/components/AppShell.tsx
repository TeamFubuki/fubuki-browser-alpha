import type { JSX } from "solid-js";

type Props = {
  children: JSX.Element;
};

export default function AppShell(props: Props) {
  return <main class="app-shell">{props.children}</main>;
}
