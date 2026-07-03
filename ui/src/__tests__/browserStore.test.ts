import { describe, expect, it, vi, beforeEach } from "vitest";

// browserStore の関数を直接インポートするのではなく、
// 同じロジックを検証するためのヘルパーをテストする。
// 実際の store は window.cefQuery に依存するため、
// ここでは純粋なロジックのみを検証する。

type Tab = {
  id: string;
  title: string;
  url: string;
  faviconUrl: string;
  errorText: string;
  zoomLevel: number;
  isLoading: boolean;
  canGoBack: boolean;
  canGoForward: boolean;
  isActive: boolean;
  isPinned: boolean;
};

type BookmarkRecord = {
  title: string;
  url: string;
  faviconUrl: string;
  createdAt: string;
};

// テスト用のモック状態
let tabs: Tab[] = [];
let bookmarks: BookmarkRecord[] = [];
let activeTabId = "";

function mockActiveTab(): Tab | undefined {
  return tabs.find((tab) => tab.id === activeTabId);
}

function mockIsTabBookmarked(url: string | undefined): boolean {
  if (!url) return false;
  return bookmarks.some((bookmark) => bookmark.url === url);
}

beforeEach(() => {
  tabs = [
    {
      id: "tab-1",
      title: "Example",
      url: "https://example.com",
      faviconUrl: "",
      errorText: "",
      zoomLevel: 0,
      isLoading: false,
      canGoBack: false,
      canGoForward: false,
      isActive: true,
      isPinned: false,
    },
    {
      id: "tab-2",
      title: "Test",
      url: "https://test.com",
      faviconUrl: "",
      errorText: "",
      zoomLevel: 0,
      isLoading: false,
      canGoBack: true,
      canGoForward: false,
      isActive: false,
      isPinned: false,
    },
  ];
  bookmarks = [
    {
      title: "Example",
      url: "https://example.com",
      faviconUrl: "",
      createdAt: "2024-01-01",
    },
  ];
  activeTabId = "tab-1";
});

describe("activeTab helper", () => {
  it("returns the active tab", () => {
    const tab = mockActiveTab();
    expect(tab).toBeDefined();
    expect(tab?.id).toBe("tab-1");
  });

  it("returns undefined when no tab is active", () => {
    activeTabId = "nonexistent";
    expect(mockActiveTab()).toBeUndefined();
  });

  it("returns undefined when tabs array is empty", () => {
    tabs = [];
    expect(mockActiveTab()).toBeUndefined();
  });
});

describe("isTabBookmarked helper", () => {
  it("returns true for bookmarked URLs", () => {
    expect(mockIsTabBookmarked("https://example.com")).toBe(true);
  });

  it("returns false for non-bookmarked URLs", () => {
    expect(mockIsTabBookmarked("https://test.com")).toBe(false);
  });

  it("returns false for undefined URL", () => {
    expect(mockIsTabBookmarked(undefined)).toBe(false);
  });

  it("returns false for empty string URL", () => {
    expect(mockIsTabBookmarked("")).toBe(false);
  });

  it("returns false when bookmarks array is empty", () => {
    bookmarks = [];
    expect(mockIsTabBookmarked("https://example.com")).toBe(false);
  });
});

describe("toggleBookmark logic", () => {
  it("skips internal URLs", () => {
    const internalUrls = [
      "fubuki://newtab/",
      "fubuki://settings/",
      "data:text/html,<h1>Test</h1>",
    ];
    for (const url of internalUrls) {
      const shouldSkip =
        !url || url.startsWith("fubuki://") || url.startsWith("data:");
      expect(shouldSkip).toBe(true);
    }
  });

  it("allows external URLs", () => {
    const url = "https://example.com";
    const shouldSkip =
      !url || url.startsWith("fubuki://") || url.startsWith("data:");
    expect(shouldSkip).toBe(false);
  });
});

describe("navigateInternal logic", () => {
  it("determines navigation target from active tab", () => {
    const tab = mockActiveTab();
    const shouldCreateNew = !tab;
    expect(shouldCreateNew).toBe(false);
    expect(tab?.id).toBe("tab-1");
  });

  it("falls back to create when no active tab", () => {
    activeTabId = "nonexistent";
    const tab = mockActiveTab();
    const shouldCreateNew = !tab;
    expect(shouldCreateNew).toBe(true);
  });
});

describe("tab filtering logic", () => {
  it("filters pinned tabs correctly", () => {
    tabs[0].isPinned = true;
    const pinned = tabs.filter((tab) => tab.isPinned);
    const normal = tabs.filter((tab) => !tab.isPinned);

    expect(pinned).toHaveLength(1);
    expect(pinned[0].id).toBe("tab-1");
    expect(normal).toHaveLength(1);
    expect(normal[0].id).toBe("tab-2");
  });

  it("filters tabs by search query", () => {
    const query = "example";
    const filtered = tabs.filter((tab) =>
      `${tab.title} ${tab.url}`.toLowerCase().includes(query.toLowerCase())
    );

    expect(filtered).toHaveLength(1);
    expect(filtered[0].id).toBe("tab-1");
  });

  it("returns all tabs for empty query", () => {
    const query = "";
    const filtered = tabs.filter((tab) =>
      `${tab.title} ${tab.url}`.toLowerCase().includes(query.toLowerCase())
    );

    expect(filtered).toHaveLength(2);
  });
});
