import type { Tab } from './bridge/fubuki';

export function tabIdForNumberShortcut(
  tabs: readonly Tab[],
  key: string,
): string | undefined {
  if (!/^[1-9]$/.test(key) || tabs.length === 0) return undefined;
  const number = Number(key);
  const index = number === 9 ? tabs.length - 1 : number - 1;
  return tabs[index]?.id;
}
