import { createEffect, createSignal } from "solid-js";

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
        spellcheck={false}
        onInput={(event) => {
          setValue(event.currentTarget.value);
          props.onDraft(event.currentTarget.value);
        }}
      />
    </form>
  );
}
