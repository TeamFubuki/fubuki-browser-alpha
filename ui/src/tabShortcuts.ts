export type ShortcutTab = { id: string };

export function tabIdForNumberShortcut(
  tabs: readonly ShortcutTab[],
  key: string,
): string | undefined {
  if (!/^[1-9]$/.test(key) || tabs.length === 0) return undefined;
  const index = key === '9' ? tabs.length - 1 : Number(key) - 1;
  return tabs[index]?.id;
}

export function adjacentTabId(
  tabs: readonly ShortcutTab[],
  activeTabId: string,
  forward: boolean,
): string | undefined {
  if (tabs.length === 0) return undefined;
  const current = tabs.findIndex((tab) => tab.id === activeTabId);
  const start = current < 0 ? 0 : current;
  const next = forward
    ? (start + 1) % tabs.length
    : (start + tabs.length - 1) % tabs.length;
  return tabs[next]?.id;
}
