import { createSignal } from 'solid-js';
import { t } from '../../i18n';
import { browserState } from '../../stores/browserStore';
import { invokeBridge } from '../../bridge/fubuki';
import SettingsDetail from './SettingsDetail';

interface Props {
  onBack: () => void;
}

export default function SidebarSettings(props: Props) {
  const lang = () => browserState.settings.language;

  const isVisible = () => browserState.settings.sidebarVisible === 'show';
  const [widthValue, setWidthValue] = createSignal(
    Number(browserState.settings.sidebarWidth) || 196,
  );

  const updateVisibility = async (checked: boolean) => {
    const value = checked ? 'show' : 'hide';
    await invokeBridge('settings.set', { key: 'sidebarVisible', value });
  };

  const updateWidth = async (value: number) => {
    setWidthValue(value);
    await invokeBridge('settings.set', {
      key: 'sidebarWidth',
      value: String(value),
    });
  };

  return (
    <SettingsDetail
      icon="📐"
      titleKey="settings.sidebar"
      descriptionKey="settings.sidebar.description"
      onBack={props.onBack}
    >
      <div class="settings-card">
        <div class="settings-card-row">
          <div class="settings-card-label">
            {t('settings.sidebarVisibility', lang())}
          </div>
          <label class="toggle-switch">
            <input
              type="checkbox"
              checked={isVisible()}
              onChange={(e) => void updateVisibility(e.currentTarget.checked)}
            />
            <span class="toggle-track" />
            <span class="toggle-thumb" />
          </label>
        </div>
      </div>

      <div class="settings-card">
        <div class="setting-row" style={{ 'margin-bottom': '0' }}>
          <div style={{ display: 'flex', 'align-items': 'center', 'justify-content': 'space-between' }}>
            <label class="setting-label" for="setting-sidebar-width">
              {t('settings.sidebarWidth', lang())}
            </label>
            <span class="setting-range-value">
              {widthValue()}{t('settings.px', lang())}
            </span>
          </div>
          <input
            id="setting-sidebar-width"
            class="setting-range"
            type="range"
            min="120"
            max="320"
            step="4"
            value={widthValue()}
            onInput={(e) => setWidthValue(Number(e.currentTarget.value))}
            onChange={(e) => void updateWidth(Number(e.currentTarget.value))}
          />
        </div>
      </div>
    </SettingsDetail>
  );
}
