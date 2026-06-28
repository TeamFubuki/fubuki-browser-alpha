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
    startupBehavior: string;
  };
  profilePath: string;
};

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
