import { t } from '../../i18n';
import { browserState } from '../../stores/browserStore';
import { invokeBridge } from '../../bridge/fubuki';
import SettingsDetail from './SettingsDetail';

interface Props {
  onBack: () => void;
}

export default function AppearanceSettings(props: Props) {
  const lang = () => browserState.settings.language;

  const updateAppearance = async (value: string) => {
    await invokeBridge('settings.set', { key: 'appearance', value });
  };

  const updateLanguage = async (value: string) => {
    await invokeBridge('settings.set', { key: 'language', value });
  };

  return (
    <SettingsDetail
      icon="🎨"
      titleKey="settings.appearance"
      descriptionKey="settings.appearance.description"
      onBack={props.onBack}
    >
      <div class="settings-card">
        <div class="settings-card-row">
          <div>
            <div class="settings-card-label">
              {t('settings.theme', lang())}
            </div>
          </div>
          <select
            class="setting-select"
            style={{ width: 'auto', 'min-width': '140px' }}
            value={browserState.settings.appearance}
            onChange={(e) => void updateAppearance(e.currentTarget.value)}
          >
            <option value="system">{t('settings.themeSystem', lang())}</option>
            <option value="light">{t('settings.themeLight', lang())}</option>
            <option value="dark">{t('settings.themeDark', lang())}</option>
          </select>
        </div>
      </div>

      <div class="settings-card">
        <div class="settings-card-row">
          <div>
            <div class="settings-card-label">
              {t('settings.language', lang())}
            </div>
          </div>
          <select
            class="setting-select"
            style={{ width: 'auto', 'min-width': '140px' }}
            value={browserState.settings.language}
            onChange={(e) => void updateLanguage(e.currentTarget.value)}
          >
            <option value="system">{t('language.system', lang())}</option>
            <option value="en">{t('language.en', lang())}</option>
            <option value="ja">{t('language.ja', lang())}</option>
          </select>
        </div>
      </div>
    </SettingsDetail>
  );
}
