import { t } from '../../../i18n';
import { browserState } from '../../../stores/browserStore';
import { ActionButton, SelectInput, SettingRow, SettingsGroup, TextInput } from '../settings-ui';
import { resetSettings, setSetting } from '../settingsActions';

export default function GeneralPanel() {
  const lang = () => browserState.settings.language;

  return (
    <div class="settings-panel">
      <SettingsGroup
        title={t('settings.general', lang())}
        description={t('settings.general.description', lang())}
        actions={
          <ActionButton
            variant="quiet"
            onClick={() => void resetSettings(['homepage', 'newTabPage', 'language'])}
          >
            {t('settings.reset', lang())}
          </ActionButton>
        }
      >
        <SettingRow
          label={t('settings.homepage', lang())}
          description={
            lang() === 'ja'
              ? 'ホームボタンとホーム指定の新規タブで開くURL'
              : 'Used by Home and by new tabs configured to open Home.'
          }
          control={
            <TextInput
              type="url"
              value={browserState.settings.homepage}
              placeholder={t('settings.homepagePlaceholder', lang())}
              onCommit={(value) => void setSetting('homepage', value)}
            />
          }
        />
        <SettingRow
          label={t('settings.newTabPage', lang())}
          control={
            <SelectInput
              label={t('settings.newTabPage', lang())}
              value={browserState.settings.newTabPage}
              options={[
                { value: 'blank', label: t('settings.newTabBlank', lang()) },
                { value: 'home', label: t('settings.newTabHome', lang()) },
              ]}
              onChange={(value) => void setSetting('newTabPage', value)}
            />
          }
        />
        <SettingRow
          label={t('settings.language', lang())}
          control={
            <SelectInput
              label={t('settings.language', lang())}
              value={browserState.settings.language}
              options={[
                { value: 'system', label: t('language.system', lang()) },
                { value: 'en', label: t('language.en', lang()) },
                { value: 'ja', label: t('language.ja', lang()) },
              ]}
              onChange={(value) => void setSetting('language', value)}
            />
          }
        />
      </SettingsGroup>
    </div>
  );
}
