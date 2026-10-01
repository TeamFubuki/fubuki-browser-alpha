export const pages = ["newtab", "history", "bookmarks", "downloads", "settings", "debug"] as const;
export type Page = (typeof pages)[number];
export function resolvePage(url: URL): Page {
  const candidate = url.protocol === "fubuki:" ? url.hostname : url.searchParams.get("page");
  return pages.find((page) => page === candidate) ?? "newtab";
}
export interface PageRecord {
  title: string;
  url: string;
  faviconUrl: string;
  path: string;
  state: string;
  percent: number;
  createdAt: string;
  downloadId: string;
}
export interface Permission {
  origin: string;
  permission: string;
  value: string;
  createdAt: string;
}
export interface PageData {
  language: string;
  appearance: string;
  settings: Record<string, string>;
  records: PageRecord[];
  permissions: Permission[];
  profilePath?: string;
  windows?: { id: string; isPrivate: boolean; tabs: { title: string; isActive: boolean }[] }[];
  commands?: { id: string; title: string; shortcut: string }[];
  events?: { name: string; message: string }[];
}
export async function loadPageData(): Promise<PageData> {
  const response = await fetch("./data.json", { cache: "no-store" });
  if (!response.ok) throw new Error("The internal database is temporarily unavailable.");
  return response.json() as Promise<PageData>;
}
export function actionUrl(params: Record<string, string>, page: Page): string {
  return `fubuki://settings/set?${new URLSearchParams({ ...params, return: `fubuki://${page}/` })}`;
}
export function safeLink(value: string): string | undefined {
  try {
    const url = new URL(value);
    if (["http:", "https:", "file:", "fubuki:"].includes(url.protocol)) return url.href;
  } catch {
    /* Malformed stored URLs are displayed as text. */
  }
  return undefined;
}
export function formatTime(value: string, dayOnly = false): string {
  const date = /^\d+$/.test(value) ? new Date(Number(value) * 1000) : new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return dayOnly ? date.toLocaleDateString() : date.toLocaleString();
}
export function downloadState(record: PageRecord): string {
  return record.state === "in_progress" && record.percent >= 100
    ? "completed"
    : record.state || "unknown";
}
