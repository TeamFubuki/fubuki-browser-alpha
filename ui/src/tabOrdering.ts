export function reorderTabById<T extends { id: string }>(
  items: readonly T[],
  tabId: string,
  toIndex: number,
): T[] {
  const fromIndex = items.findIndex((item) => item.id === tabId);
  if (fromIndex < 0 || toIndex < 0 || toIndex >= items.length) {
    return [...items];
  }
  const reordered = [...items];
  const [item] = reordered.splice(fromIndex, 1);
  reordered.splice(toIndex, 0, item);
  return reordered;
}
