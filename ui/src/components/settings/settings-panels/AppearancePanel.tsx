import { createSignal } from 'solid-js';
import { t } from '../../../i18n';
import { browserState } from '../../../stores/browserStore';
import {
  ActionButton,
  RangeSlider,
  SelectInput,
  SettingRow,
  SettingsGroup,
  Toggle,
} from '../settings-ui';
import { resetSettings, setSetting } from '../settingsActions';

export default function AppearancePanel() {
  const lang = () => browserState.settings.language;
  const [sidebarWidth, setSidebarWidth] = createSignal(
    Number(browserState.settings.sidebarWidth) || 196,
  );
  const sidebarVisible = () => browserState.settings.sidebarVisible === 'show';

  return (
    <div class="settings-panel">
      <SettingsGroup
        title={t('settings.appearance', lang())}
        description={t('settings.appearance.description', lang())}
        actions={
          <ActionButton
            variant="quiet"
            onClick={() =>
              void resetSettings(['appearance', 'sidebarVisible', 'sidebarWidth'])
            }
          >
            {t('settings.reset', lang())}
          </ActionButton>
        }
      >
        <SettingRow
          label={t('settings.theme', lang())}
          control={
            <SelectInput
              label={t('settings.theme', lang())}
              value={browserState.settings.appearance}
              options={[
                { value: 'system', label: t('settings.themeSystem', lang()) },
                { value: 'light', label: t('settings.themeLight', lang()) },
                { value: 'dark', label: t('settings.themeDark', lang()) },
              ]}
              onChange={(value) => void setSetting('appearance', value)}
            />
          }
        />
        <SettingRow
          label={t('settings.sidebarVisibility', lang())}
          description={
            lang() === 'ja'
              ? 'ブラウザ左側のタブサイドバーを表示します'
              : 'Shows the tab sidebar on the left edge of the browser.'
          }
          control={
            <Toggle
              label={t('settings.sidebarVisibility', lang())}
              checked={sidebarVisible()}
              onChange={(checked) =>
                void setSetting('sidebarVisible', checked ? 'show' : 'hide')
              }
            />
          }
        />
        <SettingRow
          label={t('settings.sidebarWidth', lang())}
          control={
            <RangeSlider
              min={160}
              max={320}
              step={4}
              value={sidebarWidth()}
              label={t('settings.sidebarWidth', lang())}
              displayValue={`${sidebarWidth()}${t('settings.px', lang())}`}
              onInput={setSidebarWidth}
              onCommit={(value) => void setSetting('sidebarWidth', String(value))}
            />
          }
        />
      </SettingsGroup>
    </div>
  );
}
