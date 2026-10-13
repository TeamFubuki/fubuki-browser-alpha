import { afterEach, describe, expect, it, vi } from "vitest";
import {
  actionUrl,
  downloadState,
  formatTime,
  loadPageData,
  pages,
  resolvePage,
  safeLink,
  type PageRecord,
} from "./data";

afterEach(() => vi.unstubAllGlobals());
describe("internal page navigation", () => {
  it("resolves every native origin and local preview route", () => {
    for (const page of pages) {
      expect(resolvePage(new URL(`fubuki://${page}/`))).toBe(page);
      expect(resolvePage(new URL(`http://localhost:5173/?page=${page}`))).toBe(page);
    }
  });
  it("falls back safely for an unknown route", () => {
    expect(resolvePage(new URL("fubuki://unknown/"))).toBe("newtab");
  });
  it("encodes action values and always returns to a known page", () => {
    const value = "https://example.com/?q=日本語&return=javascript:alert(1)";
    const url = new URL(
      actionUrl({ key: "removeBookmark", value, return: "https://evil.example/" }, "bookmarks"),
    );
    expect(url.searchParams.get("value")).toBe(value);
    expect(url.searchParams.get("return")).toBe("fubuki://bookmarks/");
    expect(url.hostname).toBe("settings");
    expect(url.pathname).toBe("/set");
  });
  it("does not turn stored script or data URLs into clickable links", () => {
    expect(safeLink("javascript:alert(1)")).toBeUndefined();
    expect(safeLink("data:text/html,<script>alert(1)</script>")).toBeUndefined();
    expect(safeLink("not a URL")).toBeUndefined();
    expect(safeLink("https://example.com/")).toBe("https://example.com/");
  });
});
describe("page data", () => {
  it("loads data from the current origin without HTTP caching", async () => {
    const data = { language: "ja", records: [] };
    const fetch = vi.fn().mockResolvedValue({ ok: true, json: async () => data });
    vi.stubGlobal("fetch", fetch);
    expect(await loadPageData()).toEqual(data);
    expect(fetch).toHaveBeenCalledWith("./data.json", { cache: "no-store" });
  });
  it("surfaces database errors instead of displaying an empty list", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({ ok: false, status: 503 }));
    await expect(loadPageData()).rejects.toThrow("temporarily unavailable");
  });
  it("normalizes completed downloads while retaining canceled states", () => {
    expect(downloadState({ state: "in_progress", percent: 100 } as PageRecord)).toBe("completed");
    expect(downloadState({ state: "canceled", percent: 10 } as PageRecord)).toBe("canceled");
  });
  it("renders epoch and legacy dates without breaking on invalid values", () => {
    expect(formatTime("1700000000")).not.toBe("Invalid Date");
    expect(formatTime("2024-01-01", true)).not.toBe("Invalid Date");
    expect(formatTime("unknown")).toBe("unknown");
  });
});
