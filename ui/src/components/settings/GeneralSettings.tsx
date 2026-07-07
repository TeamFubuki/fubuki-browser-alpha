import { t } from '../../i18n';
import { browserState } from '../../stores/browserStore';
import { invokeBridge } from '../../bridge/fubuki';
import SettingsDetail from './SettingsDetail';

interface Props {
  onBack: () => void;
}

export default function GeneralSettings(props: Props) {
  const lang = () => browserState.settings.language;

  const updateHomepage = async (value: string) => {
    await invokeBridge('settings.set', { key: 'homepage', value });
  };

  const updateNewTabPage = async (value: string) => {
    await invokeBridge('settings.set', { key: 'newTabPage', value });
  };

  return (
    <SettingsDetail
      icon="🏠"
      titleKey="settings.general"
      descriptionKey="settings.general.description"
      onBack={props.onBack}
    >
      <div class="settings-card">
        <div class="setting-row">
          <label class="setting-label" for="setting-homepage">
            {t('settings.homepage', lang())}
          </label>
          <input
            id="setting-homepage"
            class="setting-input"
            type="url"
            value={browserState.settings.homepage}
            placeholder={t('settings.homepagePlaceholder', lang())}
            onBlur={(e) => void updateHomepage(e.currentTarget.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') (e.target as HTMLInputElement).blur();
            }}
          />
        </div>
      </div>

      <div class="settings-card">
        <div class="settings-card-row">
          <div>
            <div class="settings-card-label">
              {t('settings.newTabPage', lang())}
            </div>
          </div>
          <select
            class="setting-select"
            style={{ width: 'auto', 'min-width': '120px' }}
            value={browserState.settings.newTabPage}
            onChange={(e) => void updateNewTabPage(e.currentTarget.value)}
          >
            <option value="blank">{t('settings.newTabBlank', lang())}</option>
            <option value="home">{t('settings.newTabHome', lang())}</option>
          </select>
        </div>
      </div>
    </SettingsDetail>
  );
}
