import { Show } from 'solid-js';
import { t } from '../../../i18n';
import { browserState } from '../../../stores/browserStore';
import { ActionButton, SelectInput, SettingRow, SettingsGroup, TextInput } from '../settings-ui';
import { resetSettings, setSetting } from '../settingsActions';

const searchEngines = [
  { value: 'google', label: 'Google' },
  { value: 'bing', label: 'Bing' },
  { value: 'duckduckgo', label: 'DuckDuckGo' },
  { value: 'yahoo', label: 'Yahoo!' },
  { value: 'brave', label: 'Brave' },
  { value: 'ecosia', label: 'Ecosia' },
  { value: 'kagi', label: 'Kagi' },
  { value: 'perplexity', label: 'Perplexity' },
  { value: 'custom', label: 'Custom' },
];

export default function SearchPanel() {
  const lang = () => browserState.settings.language;

  return (
    <div class="settings-panel">
      <SettingsGroup
        title={t('settings.search', lang())}
        description={t('settings.search.description', lang())}
        actions={
          <ActionButton
            variant="quiet"
            onClick={() => void resetSettings(['searchEngine', 'customSearchUrl'])}
          >
            {t('settings.reset', lang())}
          </ActionButton>
        }
      >
        <SettingRow
          label={t('settings.searchEngine', lang())}
          control={
            <SelectInput
              label={t('settings.searchEngine', lang())}
              value={browserState.settings.searchEngine}
              options={searchEngines}
              onChange={(value) => void setSetting('searchEngine', value)}
            />
          }
        />
        <Show when={browserState.settings.searchEngine === 'custom'}>
          <SettingRow
            label={t('settings.customSearchUrl', lang())}
            description={
              lang() === 'ja'
                ? '{query} が検索語句に置き換わります'
                : '{query} is replaced by the search terms.'
            }
            control={
              <TextInput
                type="url"
                value={browserState.settings.customSearchUrl}
                placeholder={t('settings.customSearchUrlPlaceholder', lang())}
                onCommit={(value) => void setSetting('customSearchUrl', value)}
              />
            }
          />
        </Show>
      </SettingsGroup>
    </div>
  );
}
