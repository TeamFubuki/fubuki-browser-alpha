import { describe, expect, it } from 'vitest';
import { dictionaries } from '../i18n/dictionaries';
import { normalizeLanguage, translate } from '../i18n';

describe('i18n dictionaries', () => {
  it('keeps Japanese and English keys in sync', () => {
    expect(Object.keys(dictionaries.ja).sort()).toEqual(
      Object.keys(dictionaries.en).sort(),
    );
  });

  it('defaults unknown language setting to system', () => {
    expect(normalizeLanguage(undefined)).toBe('system');
    expect(normalizeLanguage('')).toBe('system');
    expect(normalizeLanguage('fr')).toBe('system');
  });

  it('uses natural Japanese labels for common browser actions', () => {
    expect(translate('common.searchOrEnterUrl', 'ja')).toBe(
      '検索語句またはURLを入力',
    );
    expect(translate('common.back', 'ja')).toBe('戻る');
  });
});
