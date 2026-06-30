import type { JSX } from "solid-js";

type Props = {
  children: JSX.Element;
};

export default function PanelLayer(props: Props) {
  return <div class="panel-layer">{props.children}</div>;
}
