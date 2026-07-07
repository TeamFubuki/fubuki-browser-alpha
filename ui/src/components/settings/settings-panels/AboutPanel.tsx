import { t } from '../../../i18n';
import { browserState } from '../../../stores/browserStore';
import { SettingRow, SettingsGroup } from '../settings-ui';

export default function AboutPanel() {
  const lang = () => browserState.settings.language;

  return (
    <div class="settings-panel">
      <SettingsGroup
        title={t('settings.about', lang())}
        description={t('settings.about.description', lang())}
      >
        <SettingRow
          label="Fubuki Browser Alpha"
          control={<span class="settings-static-value">0.1.0</span>}
        />
        <SettingRow
          label="Frost Protocol"
          control={<span class="settings-static-value">{browserState.bridgeVersion}</span>}
        />
        <SettingRow
          label="Profile"
          control={
            <span class="settings-static-value compact">
              {browserState.profilePath || '-'}
            </span>
          }
        />
      </SettingsGroup>
    </div>
  );
}
