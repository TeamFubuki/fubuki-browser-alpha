import { t } from '../../i18n';
import { browserState } from '../../stores/browserStore';
import { invokeBridge } from '../../bridge/fubuki';
import SettingsDetail from './SettingsDetail';

const zoomLevels = [
  { value: '-3', label: '50%' },
  { value: '-2', label: '67%' },
  { value: '-1', label: '80%' },
  { value: '0', label: '100%' },
  { value: '1', label: '120%' },
  { value: '2', label: '150%' },
  { value: '3', label: '200%' },
];

interface Props {
  onBack: () => void;
}

export default function ZoomSettings(props: Props) {
  const lang = () => browserState.settings.language;

  const updateZoom = async (value: string) => {
    await invokeBridge('settings.set', { key: 'defaultZoomLevel', value });
  };

  return (
    <SettingsDetail
      icon="🔎"
      titleKey="settings.zoom"
      descriptionKey="settings.zoom.description"
      onBack={props.onBack}
    >
      <div class="settings-card">
        <div class="settings-card-row">
          <div>
            <div class="settings-card-label">
              {t('settings.defaultZoom', lang())}
            </div>
          </div>
          <select
            class="setting-select"
            style={{ width: 'auto', 'min-width': '100px' }}
            value={browserState.settings.defaultZoomLevel}
            onChange={(e) => void updateZoom(e.currentTarget.value)}
          >
            {zoomLevels.map((level) => (
              <option value={level.value}>{level.label}</option>
            ))}
          </select>
        </div>
      </div>
    </SettingsDetail>
  );
}
