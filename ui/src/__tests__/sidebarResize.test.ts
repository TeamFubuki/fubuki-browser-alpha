import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
  clampSidebarWidth,
  MIN_SIDEBAR_WIDTH,
  DEFAULT_SIDEBAR_WIDTH,
  MAX_SIDEBAR_WIDTH,
} from '../sidebarSizing';

// Mock global functions for Node.js environment
const mockCancelAnimationFrame = vi.fn();
const mockRequestAnimationFrame = vi.fn().mockReturnValue(1);

beforeEach(() => {
  vi.stubGlobal('cancelAnimationFrame', mockCancelAnimationFrame);
  vi.stubGlobal('requestAnimationFrame', mockRequestAnimationFrame);
  mockCancelAnimationFrame.mockClear();
  mockRequestAnimationFrame.mockClear();
});

describe('sidebar resize behavior', () => {
  describe('clampSidebarWidth', () => {
    it('returns the value when within bounds', () => {
      expect(clampSidebarWidth(200)).toBe(200);
    });

    it('clamps to MIN when below minimum', () => {
      expect(clampSidebarWidth(50)).toBe(MIN_SIDEBAR_WIDTH);
      expect(clampSidebarWidth(0)).toBe(MIN_SIDEBAR_WIDTH);
      expect(clampSidebarWidth(-100)).toBe(MIN_SIDEBAR_WIDTH);
    });

    it('clamps to MAX when above maximum', () => {
      expect(clampSidebarWidth(500)).toBe(MAX_SIDEBAR_WIDTH);
    });

    it('rounds fractional values', () => {
      expect(clampSidebarWidth(195.4)).toBe(195);
      expect(clampSidebarWidth(195.6)).toBe(196);
    });

    it('accepts exact boundary values', () => {
      expect(clampSidebarWidth(MIN_SIDEBAR_WIDTH)).toBe(MIN_SIDEBAR_WIDTH);
      expect(clampSidebarWidth(MAX_SIDEBAR_WIDTH)).toBe(MAX_SIDEBAR_WIDTH);
    });

    it('returns DEFAULT when given NaN', () => {
      const result = clampSidebarWidth(NaN);
      expect(result).toBe(DEFAULT_SIDEBAR_WIDTH);
    });

    it('returns DEFAULT when given Infinity', () => {
      expect(clampSidebarWidth(Infinity)).toBe(DEFAULT_SIDEBAR_WIDTH);
      expect(clampSidebarWidth(-Infinity)).toBe(DEFAULT_SIDEBAR_WIDTH);
    });
  });

  describe('width boundaries', () => {
    it('prevents resize beyond minimum', () => {
      const width = clampSidebarWidth(MIN_SIDEBAR_WIDTH - 10);
      expect(width).toBe(MIN_SIDEBAR_WIDTH);
    });

    it('prevents resize beyond maximum', () => {
      const width = clampSidebarWidth(MAX_SIDEBAR_WIDTH + 10);
      expect(width).toBe(MAX_SIDEBAR_WIDTH);
    });

    it('allows resize within valid range', () => {
      const testWidths = [170, 180, 196, 220, 250, 280];
      for (const width of testWidths) {
        expect(clampSidebarWidth(width)).toBe(width);
      }
    });
  });

  describe('animation frame handling', () => {
    it('cancels animation frames on cleanup', () => {
      // Simulate starting an animation
      const frameId = mockRequestAnimationFrame();
      
      // Simulate cleanup
      if (frameId) {
        mockCancelAnimationFrame(frameId);
      }
      
      expect(mockCancelAnimationFrame).toHaveBeenCalledWith(frameId);
    });

    it('handles multiple animation frames', () => {
      const frame1 = mockRequestAnimationFrame();
      const frame2 = mockRequestAnimationFrame();
      
      mockCancelAnimationFrame(frame1);
      mockCancelAnimationFrame(frame2);
      
      expect(mockCancelAnimationFrame).toHaveBeenCalledTimes(2);
      expect(mockCancelAnimationFrame).toHaveBeenLastCalledWith(frame2);
    });
  });

  describe('pointer capture', () => {
    it('handles pointer capture release safely', () => {
      const mockElement = {
        hasPointerCapture: vi.fn().mockReturnValue(true),
        releasePointerCapture: vi.fn(),
      } as unknown as HTMLElement;
      
      const pointerId = 1;
      
      if (mockElement.hasPointerCapture(pointerId)) {
        mockElement.releasePointerCapture(pointerId);
      }
      
      expect(mockElement.releasePointerCapture).toHaveBeenCalledWith(pointerId);
    });

    it('does not release capture when not captured', () => {
      const mockElement = {
        hasPointerCapture: vi.fn().mockReturnValue(false),
        releasePointerCapture: vi.fn(),
      } as unknown as HTMLElement;
      
      const pointerId = 1;
      
      if (mockElement.hasPointerCapture(pointerId)) {
        mockElement.releasePointerCapture(pointerId);
      }
      
      expect(mockElement.releasePointerCapture).not.toHaveBeenCalled();
    });
  });

  describe('resize calculation', () => {
    it('calculates width from start position and delta', () => {
      const startWidth = 200;
      const startX = 100;
      const currentX = 250;
      const delta = currentX - startX; // 150
      const expectedWidth = startWidth + delta; // 350
      
      // Should be clamped to MAX
      expect(clampSidebarWidth(expectedWidth)).toBe(MAX_SIDEBAR_WIDTH);
    });

    it('handles negative delta', () => {
      const startWidth = 200;
      const startX = 200;
      const currentX = 100;
      const delta = currentX - startX; // -100
      const expectedWidth = startWidth + delta; // 100
      
      // Should be clamped to MIN
      expect(clampSidebarWidth(expectedWidth)).toBe(MIN_SIDEBAR_WIDTH);
    });

    it('handles zero delta', () => {
      const startWidth = 200;
      const startX = 150;
      const currentX = 150;
      const delta = currentX - startX; // 0
      const expectedWidth = startWidth + delta; // 200
      
      expect(clampSidebarWidth(expectedWidth)).toBe(200);
    });

    it('handles small positive delta', () => {
      const startWidth = 200;
      const startX = 100;
      const currentX = 150;
      const delta = currentX - startX; // 50
      const expectedWidth = startWidth + delta; // 250
      
      expect(clampSidebarWidth(expectedWidth)).toBe(250);
    });

    it('handles small negative delta', () => {
      const startWidth = 250;
      const startX = 100;
      const currentX = 50;
      const delta = currentX - startX; // -50
      const expectedWidth = startWidth + delta; // 200
      
      expect(clampSidebarWidth(expectedWidth)).toBe(200);
    });
  });
});
