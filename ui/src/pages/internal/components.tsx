import {
  Show,
  createEffect,
  createSignal,
  onCleanup,
  onMount,
  type JSX,
} from 'solid-js';
import { fubukiLogoDataUri } from '../../assets/logo';
import {
  INTERNAL_ACTION_FEEDBACK_EVENT,
  announceInternalAction,
  invokeInternalAction,
  rememberInternalScrollPosition,
  type InternalActionFeedback,
} from './actions';
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
    <header class="internal-header motion-safe:animate-[page-enter_280ms_ease-out]">
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

export function ActionFeedback(props: { locale: InternalLocale }) {
  const [feedback, setFeedback] = createSignal<InternalActionFeedback>();
  let timer: number | undefined;
  const clear = () => {
    if (timer !== undefined) window.clearTimeout(timer);
    timer = undefined;
    setFeedback(undefined);
  };
  const receive = (event: Event) => {
    clear();
    setFeedback(
      (event as CustomEvent<InternalActionFeedback>).detail ?? {
        kind: 'error',
        message:
          props.locale === 'ja'
            ? '操作を完了できませんでした'
            : 'The action could not be completed',
      },
    );
    timer = window.setTimeout(clear, 3200);
  };
  onMount(() =>
    window.addEventListener(INTERNAL_ACTION_FEEDBACK_EVENT, receive),
  );
  onCleanup(() => {
    window.removeEventListener(INTERNAL_ACTION_FEEDBACK_EVENT, receive);
    if (timer !== undefined) window.clearTimeout(timer);
  });
  return (
    <Show when={feedback()}>
      {(item) => (
        <div
          class={`action-toast ${item().kind}`}
          role={item().kind === 'error' ? 'alert' : 'status'}
          aria-live="polite"
        >
          <span aria-hidden="true">
            {item().kind === 'success' ? '✓' : '!'}
          </span>
          {item().message}
        </div>
      )}
    </Show>
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
  children: JSX.Element;
  danger?: boolean;
  selected?: boolean;
  confirm?: string;
  disabled?: boolean;
  onSuccess?: () => void | Promise<void>;
  successMessage?: string;
}) {
  const [pending, setPending] = createSignal(false);
  const activate = async () => {
    if (pending() || props.disabled) return;
    if (props.confirm && !window.confirm(props.confirm)) return;
    setPending(true);
    try {
      await invokeInternalAction(props.keyName, props.value);
      await props.onSuccess?.();
      if (props.successMessage) {
        announceInternalAction({
          kind: 'success',
          message: props.successMessage,
        });
      }
    } catch (error) {
      announceInternalAction({
        kind: 'error',
        message:
          error instanceof Error
            ? error.message
            : 'The action could not be completed.',
      });
    } finally {
      setPending(false);
    }
  };
  return (
    <button
      type="button"
      disabled={props.disabled || pending()}
      aria-pressed={props.selected}
      aria-busy={pending()}
      class={`internal-button${props.danger ? ' danger' : ''}${props.selected ? ' selected' : ''}`}
      onPointerDown={rememberInternalScrollPosition}
      onKeyDown={(event) => {
        if (event.key === 'Enter' || event.key === ' ')
          rememberInternalScrollPosition();
      }}
      onClick={() => void activate()}
    >
      <Show when={pending()}>
        <span class="button-spinner" aria-hidden="true" />
      </Show>
      {props.children}
    </button>
  );
}

export function SettingChoice(props: {
  keyName: string;
  value: string;
  label: string;
  selected?: boolean;
  onSuccess?: () => void | Promise<void>;
}) {
  return (
    <ActionButton
      keyName={props.keyName}
      value={props.value}
      selected={props.selected}
      onSuccess={props.onSuccess}
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
  onSuccess?: () => void | Promise<void>;
}) {
  const [value, setValue] = createSignal(props.value);
  const [savedValue, setSavedValue] = createSignal(props.value);
  const [pending, setPending] = createSignal(false);
  createEffect(() => {
    if (!pending() && value() === savedValue()) {
      setValue(props.value);
      setSavedValue(props.value);
    }
  });
  const save = async (event: SubmitEvent) => {
    event.preventDefault();
    if (pending() || value() === savedValue()) return;
    rememberInternalScrollPosition();
    setPending(true);
    try {
      await invokeInternalAction(props.keyName, value());
      setSavedValue(value());
      await props.onSuccess?.();
      announceInternalAction({
        kind: 'success',
        message: t(props.locale, 'saved'),
      });
    } catch (error) {
      announceInternalAction({
        kind: 'error',
        message:
          error instanceof Error
            ? error.message
            : t(props.locale, 'actionFailed'),
      });
    } finally {
      setPending(false);
    }
  };
  return (
    <form class="inline-form" onSubmit={save}>
      <label>
        <span>{props.label}</span>
        <input
          type={props.type ?? 'text'}
          min={props.min}
          max={props.max}
          value={value()}
          onInput={(event) => setValue(event.currentTarget.value)}
          placeholder={props.placeholder}
        />
      </label>
      <button
        class="internal-button primary"
        type="submit"
        disabled={pending() || value() === savedValue()}
        aria-busy={pending()}
      >
        <Show when={pending()}>
          <span class="button-spinner" aria-hidden="true" />
        </Show>
        {value() === savedValue()
          ? t(props.locale, 'saved')
          : t(props.locale, 'save')}
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
