import type { Tab } from '../bridge/fubuki';

export type TabNavigationKey = 'ArrowUp' | 'ArrowDown' | 'Home' | 'End';

export function navigationTarget(
  ids: readonly string[],
  currentId: string,
  key: TabNavigationKey,
): string | null {
  const index = ids.indexOf(currentId);
  if (index < 0 || ids.length === 0) return null;
  if (key === 'Home') return ids[0];
  if (key === 'End') return ids[ids.length - 1];
  const delta = key === 'ArrowDown' ? 1 : -1;
  return ids[(index + delta + ids.length) % ids.length] ?? null;
}

export function tabIndexFor(
  id: string,
  visibleIds: readonly string[],
  activeId: string,
  focusedId: string,
): 0 | -1 {
  if (visibleIds.length === 0) return -1;
  const rovingId = visibleIds.includes(focusedId)
    ? focusedId
    : visibleIds.includes(activeId)
      ? activeId
      : visibleIds[0];
  return id === rovingId ? 0 : -1;
}

export function focusAfterClose(
  ids: readonly string[],
  closedIndex: number,
): string | null {
  if (ids.length <= 1 || closedIndex < 0 || closedIndex >= ids.length)
    return null;
  const remaining = ids.filter((_, index) => index !== closedIndex);
  return remaining[closedIndex === 0 ? 0 : closedIndex - 1] ?? null;
}

export function focusTargetAfterClose(
  adjacentId: string | null,
  activeId: string,
): string | null {
  return adjacentId ?? (activeId || null);
}

export function reorderTargetIndex(
  allTabs: readonly Tab[],
  visibleTabs: readonly Tab[],
  tabId: string,
  key: 'ArrowUp' | 'ArrowDown',
): number | null {
  const visibleIndex = visibleTabs.findIndex((tab) => tab.id === tabId);
  if (visibleIndex < 0) return null;
  const targetIndex = visibleIndex + (key === 'ArrowDown' ? 1 : -1);
  if (targetIndex < 0 || targetIndex >= visibleTabs.length) return null;
  const targetId = visibleTabs[targetIndex]?.id;
  if (!targetId) return null;
  const globalIndex = allTabs.findIndex((tab) => tab.id === targetId);
  return globalIndex < 0 ? null : globalIndex;
}

export function focusTabElement(tabId: string): void {
  const element = Array.from(
    document.querySelectorAll<HTMLElement>('[data-tab-id]'),
  ).find((candidate) => candidate.dataset.tabId === tabId);
  element?.focus();
}
