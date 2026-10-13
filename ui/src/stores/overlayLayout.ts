import type { BridgeMethodMap } from '../bridge/fubuki';

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

export function getOverlayParams(
  showPermissionPrompt: boolean,
  showRendererNotice: boolean,
): BridgeMethodMap['ui.setOverlayActive']['params'] {
  const active = showPermissionPrompt || showRendererNotice;
  // Native validates dimensions even when inactive, so omit the empty size.
  return {
    active,
    ...(active ? getOverlaySize(showPermissionPrompt, showRendererNotice) : {}),
  };
}
