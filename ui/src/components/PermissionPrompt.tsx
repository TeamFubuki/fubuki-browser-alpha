import { For, Show, createSignal } from 'solid-js';
import {
  permissions,
  type PermissionDecision,
  type PermissionType,
} from '../bridge/fubuki';
import { t, type I18nKey } from '../i18n';
import { browserState, permissionPrompts } from '../stores/browserStore';

const permissionLabels: Record<PermissionType, I18nKey> = {
  camera: 'permission.camera',
  microphone: 'permission.microphone',
  geolocation: 'permission.geolocation',
  notifications: 'permission.notifications',
  pointerLock: 'permission.pointerLock',
  keyboardLock: 'permission.keyboardLock',
};

export default function PermissionPrompt() {
  const [busy, setBusy] = createSignal(false);
  const lang = () => browserState.settings.language;

  const respond = (decision: PermissionDecision) => {
    const prompt = permissionPrompts()[0];
    if (!prompt || busy()) return;
    setBusy(true);
    void permissions
      .resolve(prompt.promptId, decision)
      .then((resolved) => {
        if (!resolved) setBusy(false);
      })
      .catch((error) => {
        setBusy(false);
        console.error('[Fubuki] Permission response failed:', error);
      });
  };

  return (
    <Show when={permissionPrompts()[0]}>
      {(prompt) => (
        <aside
          class="permission-prompt"
          data-private={prompt().isPrivate ? 'true' : 'false'}
          role="dialog"
          aria-live="polite"
          aria-label={t('permission.title', lang())}
        >
          <div class="permission-prompt-title">
            {t('permission.title', lang())}
          </div>
          <div class="permission-prompt-origin">{prompt().origin}</div>
          <p class="permission-prompt-message">
            <span>{prompt().origin}</span> requests access to:
          </p>
          <ul class="permission-prompt-list">
            <For each={prompt().permissions}>
              {(permission) => (
                <li>{t(permissionLabels[permission], lang())}</li>
              )}
            </For>
          </ul>
          <Show when={prompt().isPrivate}>
            <p class="permission-prompt-private">
              {t('permission.private', lang())}
            </p>
          </Show>
          <div class="permission-prompt-actions">
            <button
              type="button"
              disabled={busy()}
              onClick={() => respond('ask')}
            >
              {t('permission.notNow', lang())}
            </button>
            <button
              type="button"
              disabled={busy()}
              onClick={() => respond('block')}
            >
              {t('permission.block', lang())}
            </button>
            <button
              class="permission-allow"
              type="button"
              disabled={busy()}
              onClick={() => respond('allow')}
            >
              {t('permission.allow', lang())}
            </button>
          </div>
        </aside>
      )}
    </Show>
  );
}
