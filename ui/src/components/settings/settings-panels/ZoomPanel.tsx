import { createSignal } from 'solid-js';
import { t } from '../../../i18n';
import { browserState } from '../../../stores/browserStore';
import { ActionButton, RangeSlider, SettingRow, SettingsGroup } from '../settings-ui';
import { resetSettings, setSetting } from '../settingsActions';

const zoomLabels = new Map([
  [-3, '50%'],
  [-2, '67%'],
  [-1, '80%'],
  [0, '100%'],
  [1, '120%'],
  [2, '150%'],
  [3, '200%'],
]);

export default function ZoomPanel() {
  const lang = () => browserState.settings.language;
  const [zoom, setZoom] = createSignal(
    Number(browserState.settings.defaultZoomLevel) || 0,
  );

  return (
    <div class="settings-panel">
      <SettingsGroup
        title={t('settings.zoom', lang())}
        description={t('settings.zoom.description', lang())}
        actions={
          <ActionButton
            variant="quiet"
            onClick={() => void resetSettings(['defaultZoomLevel'])}
          >
            {t('settings.reset', lang())}
          </ActionButton>
        }
      >
        <SettingRow
          label={t('settings.defaultZoom', lang())}
          control={
            <RangeSlider
              min={-3}
              max={3}
              value={zoom()}
              label={t('settings.defaultZoom', lang())}
              displayValue={zoomLabels.get(zoom()) ?? '100%'}
              onInput={setZoom}
              onCommit={(value) => void setSetting('defaultZoomLevel', String(value))}
            />
          }
        />
      </SettingsGroup>
    </div>
  );
}
