export type Tab = {
  id: string;
  title: string;
  url: string;
  faviconUrl: string;
  errorText: string;
  isLoading: boolean;
  canGoBack: boolean;
  canGoForward: boolean;
  isActive: boolean;
};

export type BrowserRecord = {
  title?: string;
  url?: string;
  faviconUrl?: string;
  path?: string;
  state?: string;
  percent?: number;
  level?: string;
  message?: string;
  createdAt: string;
};

export type BrowserState = {
  bridgeVersion: string;
  activeTabId: string;
  tabs: Tab[];
  history: BrowserRecord[];
  bookmarks: BrowserRecord[];
  downloads: BrowserRecord[];
  logs: BrowserRecord[];
  settings: {
    homepage: string;
    downloadDirectory: string;
    searchEngine: string;
    customSearchUrl: string;
    startupBehavior: string;
    theme: string;
    appearance: "system" | "light" | "dark" | string;
    toolbarDensity: "compact" | "comfortable" | string;
    sidebarVisible: "show" | "hide" | string;
    sidebarWidth: string;
    defaultBookmarkDisplay: "sidebar" | "popover" | string;
    openBookmarkIn: "current" | "new" | string;
    showBookmarkFavicons: "on" | "off" | string;
    newTabPage: "blank" | "home" | string;
    homeUrl: string;
    askBeforeDownload: "on" | "off" | string;
    language: string;
    newTabBackgroundMode: string;
    newTabBackgroundColor: string;
    newTabBackgroundUrl: string;
  };
  profilePath: string;
};

export const fubukiLogoSvg = `<svg width="512" height="512" viewBox="0 0 512 512" fill="none" xmlns="http://www.w3.org/2000/svg">
<path d="M128 440L183.252 248.366M470 72L252.28 72C238.617 72 226.68 81.2317 223.244 94.4554L183.252 248.366M183.252 248.366H363.904" stroke="url(#paint0_linear_7_2)" stroke-width="25" stroke-linecap="round"/>
<path d="M95.6021 142.602L148.204 195.204M148.204 195.204L43.0001 195.204M148.204 195.204L95.6021 247.806M148.204 195.204V300.408M148.204 195.204L200.806 247.806M148.204 195.204V90M148.204 195.204L200.806 142.602M148.204 195.204H253.408" stroke="#1AADEB" stroke-width="5" stroke-linecap="round"/>
<defs>
<linearGradient id="paint0_linear_7_2" x1="257.282" y1="72" x2="257.282" y2="476.326" gradientUnits="userSpaceOnUse">
<stop stop-color="#FF9686"/>
<stop offset="1" stop-color="#A7ABE0"/>
</linearGradient>
</defs>
</svg>`;

export const fubukiLogoDataUri = `data:image/svg+xml,${encodeURIComponent(fubukiLogoSvg)}`;

type NativeQuery = {
  request: string;
  onSuccess: (response: string) => void;
  onFailure: (code: number, message: string) => void;
};

declare global {
  interface Window {
    cefQuery?: (query: NativeQuery) => void;
    fubuki: {
      bridgeVersion: string;
      invoke: <T = unknown>(method: string, params?: Record<string, unknown>) => Promise<T>;
      on: (eventName: string, listener: (payload: unknown) => void) => () => void;
    };
  }
}

const listeners = new Map<string, Set<(payload: unknown) => void>>();

function emit(eventName: string, payload: unknown) {
  listeners.get(eventName)?.forEach((listener) => listener(payload));
}

window.addEventListener("fubuki:event", (event) => {
  const detail = (event as CustomEvent).detail as { name?: string; payload?: unknown };
  if (detail?.name) {
    emit(detail.name, detail.payload);
  }
});

async function invoke<T = unknown>(method: string, params: Record<string, unknown> = {}): Promise<T> {
  if (!window.cefQuery) {
    throw new Error("Fubuki native bridge is not available");
  }

  return new Promise<T>((resolve, reject) => {
    window.cefQuery?.({
      request: JSON.stringify({ version: "1", method, params }),
      onSuccess: (response) => resolve(JSON.parse(response) as T),
      onFailure: (code, message) => reject(new Error(`${code}: ${message}`))
    });
  });
}

function on(eventName: string, listener: (payload: unknown) => void): () => void {
  const set = listeners.get(eventName) ?? new Set<(payload: unknown) => void>();
  set.add(listener);
  listeners.set(eventName, set);
  return () => set.delete(listener);
}

window.fubuki = {
  bridgeVersion: "1",
  invoke,
  on
};

export const fubuki = window.fubuki;
