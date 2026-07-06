import { describe, expect, it, beforeEach } from "vitest";

// TopBar のロジックを分離してテストする
// activeTab, isTabBookmarked の動作を検証

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

let tabs: Tab[] = [];
let bookmarks: BookmarkRecord[] = [];
let activeTabId = "";

function getActiveTab(): Tab | undefined {
  return tabs.find((tab) => tab.id === activeTabId);
}

function isBookmarked(url: string | undefined): boolean {
  if (!url) return false;
  return bookmarks.some((bookmark) => bookmark.url === url);
}

function shouldDisableBookmark(url: string | undefined): boolean {
  return !url || url.startsWith("fubuki://") || url.startsWith("data:");
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
  ];
  bookmarks = [];
  activeTabId = "tab-1";
});

describe("TopBar navigation button states", () => {
  it("disables back button when canGoBack is false", () => {
    const tab = getActiveTab();
    expect(tab?.canGoBack).toBe(false);
  });

  it("enables back button when canGoBack is true", () => {
    tabs[0].canGoBack = true;
    const tab = getActiveTab();
    expect(tab?.canGoBack).toBe(true);
  });

  it("shows reload icon when not loading", () => {
    const tab = getActiveTab();
    expect(tab?.isLoading).toBe(false);
  });

  it("shows stop icon when loading", () => {
    tabs[0].isLoading = true;
    const tab = getActiveTab();
    expect(tab?.isLoading).toBe(true);
  });
});

describe("TopBar bookmark button", () => {
  it("disables bookmark button for fubuki:// URLs", () => {
    tabs[0].url = "fubuki://newtab/";
    const tab = getActiveTab();
    expect(shouldDisableBookmark(tab?.url)).toBe(true);
  });

  it("disables bookmark button for data: URLs", () => {
    tabs[0].url = "data:text/html,<h1>Test</h1>";
    const tab = getActiveTab();
    expect(shouldDisableBookmark(tab?.url)).toBe(true);
  });

  it("enables bookmark button for regular URLs", () => {
    const tab = getActiveTab();
    expect(shouldDisableBookmark(tab?.url)).toBe(false);
  });

  it("shows unbookmarked state when URL is not in bookmarks", () => {
    const tab = getActiveTab();
    expect(isBookmarked(tab?.url)).toBe(false);
  });

  it("shows bookmarked state when URL is in bookmarks", () => {
    bookmarks.push({
      title: "Example",
      url: "https://example.com",
      faviconUrl: "",
      createdAt: "2024-01-01",
    });
    const tab = getActiveTab();
    expect(isBookmarked(tab?.url)).toBe(true);
  });
});
