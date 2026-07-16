export function reorderTab<T extends { id: string }>(
  tabs: T[],
  tabId: string,
  toIndex: number,
): T[] {
  const movedTab = tabs.find((tab) => tab.id === tabId);
  if (!movedTab) return tabs;
  const remaining = tabs.filter((tab) => tab.id !== tabId);
  remaining.splice(Math.min(toIndex, remaining.length), 0, movedTab);
  return remaining;
}
