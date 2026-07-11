export const MIN_SIDEBAR_WIDTH = 168;
export const DEFAULT_SIDEBAR_WIDTH = 196;
export const MAX_SIDEBAR_WIDTH = 280;
export const SIDEBAR_KEYBOARD_STEP = 8;

export function clampSidebarWidth(value: number): number {
  if (!Number.isFinite(value)) return DEFAULT_SIDEBAR_WIDTH;
  return Math.min(
    MAX_SIDEBAR_WIDTH,
    Math.max(MIN_SIDEBAR_WIDTH, Math.round(value)),
  );
}

export function sidebarWidthForKey(
  key: string,
  currentWidth: number,
): number | undefined {
  switch (key) {
    case 'ArrowLeft':
      return clampSidebarWidth(currentWidth - SIDEBAR_KEYBOARD_STEP);
    case 'ArrowRight':
      return clampSidebarWidth(currentWidth + SIDEBAR_KEYBOARD_STEP);
    case 'Home':
      return MIN_SIDEBAR_WIDTH;
    case 'End':
      return MAX_SIDEBAR_WIDTH;
    default:
      return undefined;
  }
}
