import { createEffect, createSignal } from "solid-js";
import { For } from "solid-js";
import { browserState } from "../stores/browserStore";

type Props = {
  value: string;
  onDraft: (value: string) => void;
  onSubmit: () => void;
};

export default function Omnibox(props: Props) {
  const [value, setValue] = createSignal(props.value);

  createEffect(() => {
    setValue(props.value);
    props.onDraft(props.value);
  });

  return (
    <form
      class="omnibox"
      onSubmit={(event) => {
        event.preventDefault();
        props.onSubmit();
      }}
    >
      <input
        value={value()}
        list="omnibox-suggestions"
        aria-label="Search or enter URL"
        spellcheck={false}
        onInput={(event) => {
          setValue(event.currentTarget.value);
          props.onDraft(event.currentTarget.value);
        }}
      />
      <datalist id="omnibox-suggestions">
        <For each={[...browserState.bookmarks, ...browserState.history].slice(0, 80)}>
          {(item) => <option value={item.url || ""} label={item.title || item.url || ""} />}
        </For>
      </datalist>
    </form>
  );
}
