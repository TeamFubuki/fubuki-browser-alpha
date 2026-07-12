import { describe, expect, it } from 'vitest';
import type { Tab } from '../bridge/fubuki';
import { patchTabForNavigation } from '../tabEventPatch';

const tab: Tab = {
  id: 'tab-1',
  title: 'Example',
  url: 'https://old.example',
  faviconUrl: '',
  errorText: 'previous error',
  zoomLevel: 0,
  isLoading: false,
  canGoBack: false,
  canGoForward: false,
  isActive: true,
  isPinned: false,
};

describe('patchTabForNavigation', () => {
  it('shows a navigation as loading immediately', () => {
    expect(
      patchTabForNavigation(tab, {
        type: 'started',
        tabId: 'tab-1',
        url: 'https://new.example',
      }),
    ).toMatchObject({
      url: 'https://new.example',
      isLoading: true,
      errorText: '',
    });
  });

  it('clears loading and errors when navigation finishes', () => {
    expect(
      patchTabForNavigation(
        { ...tab, isLoading: true },
        { type: 'finished', tabId: 'tab-1', url: 'https://new.example' },
      ),
    ).toMatchObject({ isLoading: false, errorText: '' });
  });

  it('keeps a navigation failure visible immediately', () => {
    expect(
      patchTabForNavigation(tab, {
        type: 'failed',
        tabId: 'tab-1',
        url: 'https://failed.example',
        errorText: 'Connection refused',
      }),
    ).toMatchObject({
      url: 'https://failed.example',
      isLoading: false,
      errorText: 'Connection refused',
    });
  });

  it('does not patch another tab', () => {
    expect(
      patchTabForNavigation(tab, {
        type: 'finished',
        tabId: 'tab-2',
        url: 'https://other.example',
      }),
    ).toBeNull();
  });
});
