import { describe, expect, it } from 'vitest';
import { getOverlayParams, getOverlaySize } from '../stores/overlayLayout';

describe('getOverlaySize', () => {
  it('uses the permission prompt size when only permissions are shown', () => {
    expect(getOverlaySize(true, false)).toEqual({ width: 392, height: 320 });
  });

  it('uses the renderer recovery size when only recovery is shown', () => {
    expect(getOverlaySize(false, true)).toEqual({ width: 520, height: 280 });
  });

  it('preserves both panels by taking the maximum dimensions', () => {
    expect(getOverlaySize(true, true)).toEqual({ width: 520, height: 320 });
  });

  it('returns an empty area when neither panel is shown', () => {
    expect(getOverlaySize(false, false)).toEqual({ width: 0, height: 0 });
  });
});

describe('native overlay request', () => {
  it('omits dimensions when dismissing the overlay so native validation accepts it', () => {
    expect(getOverlayParams(false, false)).toEqual({ active: false });
  });

  it('includes valid dimensions when recovery is displayed', () => {
    expect(getOverlayParams(false, true)).toEqual({
      active: true,
      width: 520,
      height: 280,
    });
  });
});
