import { describe, expect, it } from 'vitest';
import type { Tab } from '../bridge/fubuki';
import { tabIdForNumberShortcut } from '../appShortcuts';

const tab = (id: string): Tab => ({
  id,
  windowId: 'window-1',
  title: id,
  url: `https://${id}.test`,
  faviconUrl: '',
  errorText: '',
  zoomLevel: 0,
  isLoading: false,
  canGoBack: false,
  canGoForward: false,
  isActive: false,
  isPinned: false,
});

describe('number tab shortcuts', () => {
  const tabs = [tab('first'), tab('second'), tab('third')];

  it('maps 1 through 8 to the matching visible position', () => {
    expect(tabIdForNumberShortcut(tabs, '1')).toBe('first');
    expect(tabIdForNumberShortcut(tabs, '3')).toBe('third');
    expect(tabIdForNumberShortcut(tabs, '4')).toBeUndefined();
  });

  it('maps 9 to the last tab', () => {
    expect(tabIdForNumberShortcut(tabs, '9')).toBe('third');
  });

  it('ignores non-number keys and empty tab lists', () => {
    expect(tabIdForNumberShortcut(tabs, '0')).toBeUndefined();
    expect(tabIdForNumberShortcut([], '1')).toBeUndefined();
  });
});
