import { describe, it, expect } from "vitest";
import {
  clampSidebarWidth,
  MIN_SIDEBAR_WIDTH,
  DEFAULT_SIDEBAR_WIDTH,
  MAX_SIDEBAR_WIDTH,
} from "../sidebarSizing";

describe("clampSidebarWidth", () => {
  it("returns the value when within bounds", () => {
    expect(clampSidebarWidth(200)).toBe(200);
  });

  it("clamps to MIN when below minimum", () => {
    expect(clampSidebarWidth(50)).toBe(MIN_SIDEBAR_WIDTH);
    expect(clampSidebarWidth(0)).toBe(MIN_SIDEBAR_WIDTH);
    expect(clampSidebarWidth(-100)).toBe(MIN_SIDEBAR_WIDTH);
  });

  it("clamps to MAX when above maximum", () => {
    expect(clampSidebarWidth(500)).toBe(MAX_SIDEBAR_WIDTH);
  });

  it("rounds fractional values", () => {
    expect(clampSidebarWidth(195.4)).toBe(195);
    expect(clampSidebarWidth(195.6)).toBe(196);
  });

  it("accepts exact boundary values", () => {
    expect(clampSidebarWidth(MIN_SIDEBAR_WIDTH)).toBe(MIN_SIDEBAR_WIDTH);
    expect(clampSidebarWidth(MAX_SIDEBAR_WIDTH)).toBe(MAX_SIDEBAR_WIDTH);
  });

  it("returns DEFAULT when given NaN", () => {
    const result = clampSidebarWidth(NaN);
    expect(result).toBe(DEFAULT_SIDEBAR_WIDTH);
  });

  it("returns DEFAULT when given Infinity", () => {
    expect(clampSidebarWidth(Infinity)).toBe(DEFAULT_SIDEBAR_WIDTH);
    expect(clampSidebarWidth(-Infinity)).toBe(DEFAULT_SIDEBAR_WIDTH);
  });
});

describe("constants", () => {
  it("MIN <= DEFAULT <= MAX", () => {
    expect(MIN_SIDEBAR_WIDTH).toBeLessThanOrEqual(DEFAULT_SIDEBAR_WIDTH);
    expect(DEFAULT_SIDEBAR_WIDTH).toBeLessThanOrEqual(MAX_SIDEBAR_WIDTH);
  });

  it("MIN is positive", () => {
    expect(MIN_SIDEBAR_WIDTH).toBeGreaterThan(0);
  });
});
