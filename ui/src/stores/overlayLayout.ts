export type OverlaySize = {
  width: number;
  height: number;
};

const permissionOverlay: OverlaySize = { width: 392, height: 320 };
const rendererOverlay: OverlaySize = { width: 520, height: 280 };

export function getOverlaySize(
  showPermissionPrompt: boolean,
  showRendererNotice: boolean,
): OverlaySize {
  return {
    width: Math.max(
      showPermissionPrompt ? permissionOverlay.width : 0,
      showRendererNotice ? rendererOverlay.width : 0,
    ),
    height: Math.max(
      showPermissionPrompt ? permissionOverlay.height : 0,
      showRendererNotice ? rendererOverlay.height : 0,
    ),
  };
}
