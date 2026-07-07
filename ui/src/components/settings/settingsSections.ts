import type { I18nKey } from '../../i18n';

export type SettingsSection =
  | 'general'
  | 'appearance'
  | 'search'
  | 'zoom'
  | 'mcp'
  | 'about';

export type SettingsSectionDefinition = {
  id: SettingsSection;
  icon: string;
  titleKey: I18nKey;
  descriptionKey: I18nKey;
};

export const settingsSections: SettingsSectionDefinition[] = [
  {
    id: 'general',
    icon: 'G',
    titleKey: 'settings.general',
    descriptionKey: 'settings.general.description',
  },
  {
    id: 'appearance',
    icon: 'A',
    titleKey: 'settings.appearance',
    descriptionKey: 'settings.appearance.description',
  },
  {
    id: 'search',
    icon: 'S',
    titleKey: 'settings.search',
    descriptionKey: 'settings.search.description',
  },
  {
    id: 'zoom',
    icon: 'T',
    titleKey: 'settings.zoom',
    descriptionKey: 'settings.zoom.description',
  },
  {
    id: 'mcp',
    icon: 'M',
    titleKey: 'settings.mcp',
    descriptionKey: 'settings.mcpDescription',
  },
  {
    id: 'about',
    icon: 'i',
    titleKey: 'settings.about',
    descriptionKey: 'settings.about.description',
  },
];
