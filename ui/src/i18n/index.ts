import { dictionaries, type I18nKey, type Language } from './dictionaries';

export type LanguageSetting = 'system' | Language;

export function normalizeLanguage(value: string | undefined): LanguageSetting {
  return value === 'ja' || value === 'en' || value === 'system'
    ? value
    : 'system';
}

export function systemLanguage(): Language {
  const language = navigator.language || navigator.languages?.[0] || 'en';
  return language.toLowerCase().startsWith('ja') ? 'ja' : 'en';
}

let cachedSetting: string | undefined;
let cachedLang: Language = 'en';

export function resolveLanguage(setting: string | undefined): Language {
  if (setting === cachedSetting) return cachedLang;
  cachedSetting = setting;
  const normalized = normalizeLanguage(setting);
  cachedLang = normalized === 'system' ? systemLanguage() : normalized;
  return cachedLang;
}

export function translate(key: I18nKey, language: Language): string {
  return dictionaries[language][key] ?? dictionaries.en[key] ?? key;
}

export function t(key: I18nKey, setting?: string): string {
  return translate(key, resolveLanguage(setting));
}

export { dictionaries };
export type { I18nKey, Language };
