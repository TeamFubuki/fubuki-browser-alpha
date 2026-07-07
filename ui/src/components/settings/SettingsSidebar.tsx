import { For } from 'solid-js';
import { t } from '../../i18n';
import { browserState } from '../../stores/browserStore';
import { settingsSections, type SettingsSection } from './settingsSections';

export default function SettingsSidebar(props: {
  activeSection: SettingsSection;
  onSelect: (section: SettingsSection) => void;
}) {
  const lang = () => browserState.settings.language;

  return (
    <aside class="settings-sidebar">
      <div class="settings-sidebar-title">{t('common.settings', lang())}</div>
      <nav class="settings-sidebar-nav" aria-label={t('common.settings', lang())}>
        <For each={settingsSections}>
          {(section) => (
            <button
              type="button"
              classList={{
                'settings-nav-item': true,
                active: props.activeSection === section.id,
              }}
              onClick={() => props.onSelect(section.id)}
            >
              <span class="settings-nav-icon" aria-hidden="true">
                {section.icon}
              </span>
              <span>
                <strong>{t(section.titleKey, lang())}</strong>
                <small>{t(section.descriptionKey, lang())}</small>
              </span>
            </button>
          )}
        </For>
      </nav>
    </aside>
  );
}
