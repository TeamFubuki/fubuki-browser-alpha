import { type JSX } from 'solid-js';
import { t } from '../../i18n';
import { browserState } from '../../stores/browserStore';

interface SettingsDetailProps {
  icon: string;
  titleKey: string;
  descriptionKey?: string;
  onBack: () => void;
  children: JSX.Element;
}

export default function SettingsDetail(props: SettingsDetailProps) {
  const lang = () => browserState.settings.language;

  return (
    <>
      <header class="settings-topbar">
        <button
          class="settings-back-button"
          onClick={props.onBack}
          title={t('common.back', lang())}
          aria-label={t('common.back', lang())}
        >
          <span class="settings-back-icon" aria-hidden="true">←</span>
        </button>
        <span class="settings-topbar-title">
          {t(props.titleKey as any, lang())}
        </span>
      </header>
      <div class="settings-scroll">
        <div class="settings-scroll-inner">
          <div class="settings-detail-header">
            <span class="settings-detail-icon" aria-hidden="true">
              {props.icon}
            </span>
            <div>
              <h2 class="settings-detail-title">
                {t(props.titleKey as any, lang())}
              </h2>
              {props.descriptionKey && (
                <p class="settings-detail-description">
                  {t(props.descriptionKey as any, lang())}
                </p>
              )}
            </div>
          </div>
          {props.children}
        </div>
      </div>
    </>
  );
}
