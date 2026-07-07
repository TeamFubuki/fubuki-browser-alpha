import { t } from '../../i18n';
import { browserState } from '../../stores/browserStore';

export type SettingsSection =
  | 'general'
  | 'appearance'
  | 'search'
  | 'sidebar'
  | 'zoom'
  | 'mcp';

interface SectionDef {
  id: SettingsSection;
  icon: string;
  titleKey: string;
  descriptionKey: string;
}

const sections: SectionDef[] = [
  {
    id: 'general',
    icon: '🏠',
    titleKey: 'settings.general',
    descriptionKey: 'settings.general.description',
  },
  {
    id: 'appearance',
    icon: '🎨',
    titleKey: 'settings.appearance',
    descriptionKey: 'settings.appearance.description',
  },
  {
    id: 'search',
    icon: '🔍',
    titleKey: 'settings.search',
    descriptionKey: 'settings.search.description',
  },
  {
    id: 'sidebar',
    icon: '📐',
    titleKey: 'settings.sidebar',
    descriptionKey: 'settings.sidebar.description',
  },
  {
    id: 'zoom',
    icon: '🔎',
    titleKey: 'settings.zoom',
    descriptionKey: 'settings.zoom.description',
  },
  {
    id: 'mcp',
    icon: '🤖',
    titleKey: 'settings.mcp',
    descriptionKey: 'settings.mcp',
  },
];

interface SettingsListProps {
  onSelect: (section: SettingsSection) => void;
}

export default function SettingsList(props: SettingsListProps) {
  const lang = () => browserState.settings.language;

  return (
    <div class="settings-list">
      {sections.map((section) => (
        <button
          class="settings-category"
          onClick={() => props.onSelect(section.id)}
          aria-label={t(section.titleKey as any, lang())}
        >
          <span class="settings-category-icon" aria-hidden="true">
            {section.icon}
          </span>
          <span class="settings-category-content">
            <span class="settings-category-title">
              {t(section.titleKey as any, lang())}
            </span>
            <span class="settings-category-description">
              {section.id === 'mcp'
                ? t('settings.mcpDescription', lang())
                : t(section.descriptionKey as any, lang())}
            </span>
          </span>
          <span class="settings-category-chevron" aria-hidden="true">
            ›
          </span>
        </button>
      ))}
    </div>
  );
}
