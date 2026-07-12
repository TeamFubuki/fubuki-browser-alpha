import type { Tab } from './bridge/fubuki';

export type NavigationEvent =
  | { type: 'started'; tabId: string; url: string }
  | { type: 'finished'; tabId: string; url: string }
  | { type: 'failed'; tabId: string; url: string; errorText: string };

/** Apply renderer-visible navigation state without waiting for a snapshot. */
export function patchTabForNavigation(
  tab: Tab,
  event: NavigationEvent,
): Tab | null {
  if (tab.id !== event.tabId) return null;
  if (event.type === 'started') {
    return { ...tab, url: event.url, isLoading: true, errorText: '' };
  }
  if (event.type === 'finished') {
    return { ...tab, url: event.url, isLoading: false, errorText: '' };
  }
  return {
    ...tab,
    url: event.url,
    isLoading: false,
    errorText: event.errorText,
  };
}
