import { For, JSX, Show } from 'solid-js';

type Option = {
  value: string;
  label: string;
};

export function SettingsGroup(props: {
  title: string;
  description?: string;
  children: JSX.Element;
  actions?: JSX.Element;
}) {
  return (
    <section class="settings-group">
      <header class="settings-group-header">
        <div>
          <h2>{props.title}</h2>
          <Show when={props.description}>
            <p>{props.description}</p>
          </Show>
        </div>
        <Show when={props.actions}>
          <div class="settings-group-actions">{props.actions}</div>
        </Show>
      </header>
      <div class="settings-group-body">{props.children}</div>
    </section>
  );
}

export function SettingRow(props: {
  label: string;
  description?: string;
  control: JSX.Element;
}) {
  return (
    <div class="setting-row">
      <div class="setting-row-copy">
        <div class="setting-label">{props.label}</div>
        <Show when={props.description}>
          <p class="setting-description">{props.description}</p>
        </Show>
      </div>
      <div class="setting-row-control">{props.control}</div>
    </div>
  );
}

export function Toggle(props: {
  checked: boolean;
  disabled?: boolean;
  label: string;
  onChange: (checked: boolean) => void;
}) {
  return (
    <label class="settings-toggle" aria-label={props.label}>
      <input
        type="checkbox"
        checked={props.checked}
        disabled={props.disabled}
        onChange={(event) => props.onChange(event.currentTarget.checked)}
      />
      <span class="settings-toggle-track" />
    </label>
  );
}

export function SelectInput(props: {
  value: string;
  options: Option[];
  label: string;
  onChange: (value: string) => void;
}) {
  return (
    <select
      class="settings-select"
      aria-label={props.label}
      value={props.value}
      onChange={(event) => props.onChange(event.currentTarget.value)}
    >
      <For each={props.options}>
        {(option) => <option value={option.value}>{option.label}</option>}
      </For>
    </select>
  );
}

export function TextInput(props: {
  id?: string;
  value: string;
  placeholder?: string;
  type?: string;
  disabled?: boolean;
  onCommit: (value: string) => void;
}) {
  return (
    <input
      id={props.id}
      class="settings-input"
      type={props.type ?? 'text'}
      value={props.value}
      placeholder={props.placeholder}
      disabled={props.disabled}
      onBlur={(event) => props.onCommit(event.currentTarget.value)}
      onKeyDown={(event) => {
        if (event.key === 'Enter') event.currentTarget.blur();
      }}
    />
  );
}

export function RangeSlider(props: {
  min: number;
  max: number;
  step?: number;
  value: number;
  label: string;
  displayValue: string;
  onInput?: (value: number) => void;
  onCommit: (value: number) => void;
}) {
  return (
    <div class="settings-range-control">
      <input
        class="settings-range"
        type="range"
        min={props.min}
        max={props.max}
        step={props.step ?? 1}
        value={props.value}
        aria-label={props.label}
        onInput={(event) => props.onInput?.(Number(event.currentTarget.value))}
        onChange={(event) => props.onCommit(Number(event.currentTarget.value))}
      />
      <span class="settings-range-value">{props.displayValue}</span>
    </div>
  );
}

export function IconButton(props: {
  label: string;
  icon: string;
  disabled?: boolean;
  variant?: 'primary' | 'quiet' | 'danger';
  onClick: () => void;
}) {
  return (
    <button
      classList={{
        'settings-icon-button': true,
        primary: props.variant === 'primary',
        danger: props.variant === 'danger',
      }}
      type="button"
      title={props.label}
      aria-label={props.label}
      disabled={props.disabled}
      onClick={props.onClick}
    >
      {props.icon}
    </button>
  );
}

export function ActionButton(props: {
  children: JSX.Element;
  disabled?: boolean;
  variant?: 'primary' | 'quiet' | 'danger';
  onClick: () => void;
}) {
  return (
    <button
      classList={{
        'settings-action-button': true,
        primary: props.variant === 'primary',
        danger: props.variant === 'danger',
      }}
      type="button"
      disabled={props.disabled}
      onClick={props.onClick}
    >
      {props.children}
    </button>
  );
}

export function ToolCheckbox(props: {
  tool: string;
  description: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <label class="settings-tool-checkbox">
      <input
        type="checkbox"
        checked={props.checked}
        onChange={(event) => props.onChange(event.currentTarget.checked)}
      />
      <span>
        <strong>{props.tool}</strong>
        <small>{props.description}</small>
      </span>
    </label>
  );
}

export function JsonEditor(props: {
  value: string;
  label: string;
  onCommit?: (value: string) => void;
}) {
  return (
    <textarea
      class="settings-json-editor"
      aria-label={props.label}
      spellcheck={false}
      value={props.value}
      onBlur={(event) => props.onCommit?.(event.currentTarget.value)}
    />
  );
}
