import { describe, expect, it } from 'vitest';
import {
  buildSearchUrl,
  normalizeOmniboxInput,
  shouldTreatAsSearch,
} from '../utils/navigation';

describe('omnibox input classification', () => {
  it('treats Japanese text as search input', () => {
    expect(shouldTreatAsSearch('天気 東京')).toBe(true);
    expect(shouldTreatAsSearch('日本語 テスト')).toBe(true);
    expect(normalizeOmniboxInput('  雪 ブラウザ  ')).toEqual({
      kind: 'search',
      value: '雪 ブラウザ',
    });
  });

  it('keeps explicit URLs as URL input', () => {
    expect(shouldTreatAsSearch('https://example.com')).toBe(false);
    expect(shouldTreatAsSearch('http:example.com')).toBe(false);
    expect(shouldTreatAsSearch('about:blank')).toBe(false);
    expect(shouldTreatAsSearch('data:text/html,<h1>Hello</h1>')).toBe(false);
    expect(shouldTreatAsSearch('fubuki://settings/')).toBe(false);
    expect(normalizeOmniboxInput('example.com')).toEqual({
      kind: 'url',
      value: 'example.com',
    });
  });

  it('keeps local and LAN-like inputs as URL input', () => {
    expect(shouldTreatAsSearch('localhost')).toBe(false);
    expect(shouldTreatAsSearch('localhost:5173')).toBe(false);
    expect(shouldTreatAsSearch('127.0.0.1:3000')).toBe(false);
    expect(shouldTreatAsSearch('192.168.1.1')).toBe(false);
    expect(shouldTreatAsSearch('[::1]:5173')).toBe(false);
    expect(shouldTreatAsSearch('nas.local')).toBe(false);
  });

  it('keeps domain-like inputs as URL input', () => {
    expect(shouldTreatAsSearch('example.com/path')).toBe(false);
    expect(shouldTreatAsSearch('example.com:8080/path?q=1')).toBe(false);
    expect(shouldTreatAsSearch('日本語.com')).toBe(false);
    expect(shouldTreatAsSearch('münich.example')).toBe(false);
  });

  it('keeps plain search phrases as search input', () => {
    expect(shouldTreatAsSearch('hello world')).toBe(true);
    expect(shouldTreatAsSearch('東京 天気')).toBe(true);
    expect(shouldTreatAsSearch('github fubuki browser')).toBe(true);
  });
});

describe('search URL construction', () => {
  it('supports both custom placeholders', () => {
    expect(
      buildSearchUrl('雪 browser', 'custom', 'https://search.test/?q={query}'),
    ).toBe('https://search.test/?q=%E9%9B%AA+browser');
    expect(
      buildSearchUrl('solid js', 'custom', 'https://search.test/?term=%s'),
    ).toBe('https://search.test/?term=solid+js');
  });

  it('appends a query to custom URLs without a placeholder', () => {
    expect(buildSearchUrl('fubuki', 'custom', 'https://search.test/')).toBe(
      'https://search.test/?q=fubuki',
    );
    expect(
      buildSearchUrl('fubuki', 'custom', 'https://search.test/?safe=1'),
    ).toBe('https://search.test/?safe=1&q=fubuki');
  });
});
