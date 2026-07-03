import { describe, expect, it } from "vitest";
import { normalizeOmniboxInput, shouldTreatAsSearch } from "../utils/navigation";

describe("omnibox input classification", () => {
  it("treats Japanese text as search input", () => {
    expect(shouldTreatAsSearch("天気 東京")).toBe(true);
    expect(shouldTreatAsSearch("日本語.com")).toBe(true);
    expect(normalizeOmniboxInput("  雪 ブラウザ  ")).toEqual({ kind: "search", value: "雪 ブラウザ" });
  });

  it("keeps explicit URLs as URL input", () => {
    expect(shouldTreatAsSearch("https://example.com")).toBe(false);
    expect(shouldTreatAsSearch("fubuki://settings/")).toBe(false);
    expect(normalizeOmniboxInput("example.com")).toEqual({ kind: "url", value: "example.com" });
  });
});
