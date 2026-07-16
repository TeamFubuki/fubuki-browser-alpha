import type { JSX } from 'solid-js';
import { fubukiLogoDataUri } from '../../assets/logo';
import type { InternalLocale } from './data';
import { t } from './data';

export function Logo() {
  return <img class="internal-logo" src={fubukiLogoDataUri} alt="" />;
}

export function PageHeader(props: {
  title: string;
  eyebrow?: string;
  actions?: JSX.Element;
}) {
  return (
    <header class="internal-header">
      <div class="internal-heading">
        <Logo />
        <div>
          {props.eyebrow && (
            <span class="internal-eyebrow">{props.eyebrow}</span>
          )}
          <h1>{props.title}</h1>
        </div>
      </div>
      {props.actions && <div class="header-actions">{props.actions}</div>}
    </header>
  );
}

export function EmptyState(props: {
  title: string;
  detail?: string;
  icon?: string;
}) {
  return (
    <section class="internal-empty">
      <span class="empty-icon" aria-hidden="true">
        {props.icon ?? '✦'}
      </span>
      <strong>{props.title}</strong>
      {props.detail && <p>{props.detail}</p>}
    </section>
  );
}

export function LoadingState(props: {
  locale: InternalLocale;
  error?: boolean;
}) {
  return (
    <div class={`internal-status${props.error ? ' error' : ''}`} role="status">
      {t(props.locale, props.error ? 'loadError' : 'loading')}
    </div>
  );
}

export function SearchField(props: {
  value: string;
  placeholder: string;
  onInput: (value: string) => void;
}) {
  return (
    <label class="search-field">
      <span aria-hidden="true">⌕</span>
      <input
        type="search"
        value={props.value}
        onInput={(event) => props.onInput(event.currentTarget.value)}
        placeholder={props.placeholder}
        aria-label={props.placeholder}
      />
    </label>
  );
}

export function ActionButton(props: {
  keyName: string;
  value: string;
  returnUrl: string;
  children: string;
  danger?: boolean;
  post?: boolean;
  selected?: boolean;
  confirm?: string;
  disabled?: boolean;
}) {
  const params = new URLSearchParams({
    key: props.keyName,
    value: props.value,
    return: props.returnUrl,
  });
  const method = props.danger || props.post ? 'post' : 'get';
  return (
    <form
      method={method}
      action={`fubuki://settings/set?${params}`}
      onSubmit={(event) => {
        if (props.confirm && !window.confirm(props.confirm))
          event.preventDefault();
      }}
    >
      <input type="hidden" name="key" value={props.keyName} />
      <input type="hidden" name="value" value={props.value} />
      <input type="hidden" name="return" value={props.returnUrl} />
      <button
        type="submit"
        disabled={props.disabled}
        class={`internal-button${props.danger ? ' danger' : ''}${props.selected ? ' selected' : ''}`}
      >
        {props.children}
      </button>
    </form>
  );
}

export function SettingChoice(props: {
  keyName: string;
  value: string;
  label: string;
  selected?: boolean;
}) {
  return (
    <ActionButton
      keyName={props.keyName}
      value={props.value}
      returnUrl="fubuki://settings/"
      selected={props.selected}
    >
      {props.label}
    </ActionButton>
  );
}

export function SettingInput(props: {
  keyName: string;
  label: string;
  value: string;
  placeholder?: string;
  locale: InternalLocale;
  type?: string;
  min?: string;
  max?: string;
}) {
  return (
    <form class="inline-form" action="fubuki://settings/set" method="get">
      <input type="hidden" name="key" value={props.keyName} />
      <input type="hidden" name="return" value="fubuki://settings/" />
      <label>
        <span>{props.label}</span>
        <input
          type={props.type ?? 'text'}
          min={props.min}
          max={props.max}
          name="value"
          value={props.value}
          placeholder={props.placeholder}
        />
      </label>
      <button class="internal-button primary" type="submit">
        {t(props.locale, 'save')}
      </button>
    </form>
  );
}

export function Favicon(props: { url?: string }) {
  return (
    <span class="record-icon">
      {props.url ? (
        <img src={props.url} alt="" loading="lazy" />
      ) : (
        <span aria-hidden="true">◆</span>
      )}
    </span>
  );
}
