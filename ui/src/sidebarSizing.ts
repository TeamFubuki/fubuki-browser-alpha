export const MIN_SIDEBAR_WIDTH = 168;
export const DEFAULT_SIDEBAR_WIDTH = 196;
export const MAX_SIDEBAR_WIDTH = 280;

export function clampSidebarWidth(value: number): number {
  if (!Number.isFinite(value)) return DEFAULT_SIDEBAR_WIDTH;
  return Math.min(
    MAX_SIDEBAR_WIDTH,
    Math.max(MIN_SIDEBAR_WIDTH, Math.round(value)),
  );
}
