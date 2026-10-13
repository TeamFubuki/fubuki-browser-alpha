import type { Tab } from '../bridge/fubuki';

export type DropEdge = 'before' | 'after';
export type TabDropTarget = { tabId: string; edge: DropEdge };

// tabs.move takes the final index after the source tab has been removed.
export function tabDropIndex(
  allTabs: readonly Tab[],
  draggedId: string,
  target: TabDropTarget,
): number | null {
  const fromIndex = allTabs.findIndex((tab) => tab.id === draggedId);
  const targetIndex = allTabs.findIndex((tab) => tab.id === target.tabId);
  if (fromIndex < 0 || targetIndex < 0 || fromIndex === targetIndex)
    return null;
  const source = allTabs[fromIndex];
  const destination = allTabs[targetIndex];
  if (
    source.windowId !== destination.windowId ||
    source.isPinned !== destination.isPinned
  )
    return null;
  const index =
    targetIndex +
    (target.edge === 'after' ? 1 : 0) -
    (fromIndex < targetIndex ? 1 : 0);
  return index === fromIndex ? null : index;
}

export function tabDropEdge(
  event: Pick<PointerEvent, 'clientX' | 'clientY'>,
  rect: Pick<DOMRect, 'left' | 'top' | 'width' | 'height'>,
  pinned: boolean,
): DropEdge {
  const after = pinned
    ? event.clientX >= rect.left + rect.width / 2
    : event.clientY >= rect.top + rect.height / 2;
  return after ? 'after' : 'before';
}
