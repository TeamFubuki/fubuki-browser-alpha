import { createSignal, For } from 'solid-js';
import { t } from '../../i18n';
import { browserState } from '../../stores/browserStore';
import { invokeBridge } from '../../bridge/fubuki';
import SettingsDetail from './SettingsDetail';

const searchEngines = [
  { id: 'google', name: 'Google' },
  { id: 'bing', name: 'Bing' },
  { id: 'duckduckgo', name: 'DuckDuckGo' },
  { id: 'yahoo', name: 'Yahoo!' },
  { id: 'brave', name: 'Brave' },
  { id: 'ecosia', name: 'Ecosia' },
  { id: 'kagi', name: 'Kagi' },
  { id: 'perplexity', name: 'Perplexity' },
  { id: 'custom', name: 'Custom' },
];

interface Props {
  onBack: () => void;
}

export default function SearchSettings(props: Props) {
  const lang = () => browserState.settings.language;
  const [showCustom, setShowCustom] = createSignal(
    browserState.settings.searchEngine === 'custom',
  );

  const updateSearchEngine = async (value: string) => {
    setShowCustom(value === 'custom');
    await invokeBridge('settings.set', { key: 'searchEngine', value });
  };

  const updateCustomSearchUrl = async (value: string) => {
    await invokeBridge('settings.set', { key: 'customSearchUrl', value });
  };

  return (
    <SettingsDetail
      icon="🔍"
      titleKey="settings.search"
      descriptionKey="settings.search.description"
      onBack={props.onBack}
    >
      <div class="settings-card">
        <div class="settings-card-row">
          <div>
            <div class="settings-card-label">
              {t('settings.searchEngine', lang())}
            </div>
          </div>
          <select
            class="setting-select"
            style={{ width: 'auto', 'min-width': '140px' }}
            value={browserState.settings.searchEngine}
            onChange={(e) => void updateSearchEngine(e.currentTarget.value)}
          >
            <For each={searchEngines}>
              {(engine) => (
                <option value={engine.id}>{engine.name}</option>
              )}
            </For>
          </select>
        </div>
      </div>

      {showCustom() && (
        <div class="settings-card">
          <div class="setting-row" style={{ 'margin-bottom': '0' }}>
            <label class="setting-label" for="setting-custom-search">
              {t('settings.customSearchUrl', lang())}
            </label>
            <input
              id="setting-custom-search"
              class="setting-input"
              type="url"
              value={browserState.settings.customSearchUrl}
              placeholder={t('settings.customSearchUrlPlaceholder', lang())}
              onBlur={(e) => void updateCustomSearchUrl(e.currentTarget.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') (e.target as HTMLInputElement).blur();
              }}
            />
            <p class="setting-description">
              {'{query}'} — {lang() === 'ja' ? '検索語句が置き換えられます' : 'Search query placeholder'}
            </p>
          </div>
        </div>
      )}
    </SettingsDetail>
  );
}
