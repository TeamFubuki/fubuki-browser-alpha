import { createEffect, createSignal, onCleanup, onMount } from 'solid-js';
import { commands, tabs } from './bridge/fubuki';
import BrowserShell from './components/BrowserShell';
import CommandPalette from './components/commandPalette/CommandPalette';
import { resolveLanguage } from './i18n';
import { clampSidebarWidth, DEFAULT_SIDEBAR_WIDTH } from './sidebarSizing';
import {
  activeTab,
  bindNativeEvents,
  browserState,
  navigateInternal,
  toggleBookmark,
  toggleSidebar,
} from './stores/browserStore';

// Cached DOM references
let omniboxInput: HTMLInputElement | null = null;

function onKeyDown(event: KeyboardEvent) {
  const command = event.metaKey || event.ctrlKey;
  if (!command) return;

  const tab = activeTab();
  const key = event.key.toLowerCase();
  if (key === 'k') {
    window.dispatchEvent(new CustomEvent('fubuki:open-palette'));
    event.preventDefault();
    return;
  }
  if (key === 'l') {
    if (!omniboxInput) omniboxInput = document.querySelector('.omnibox-input');
    omniboxInput?.focus();
    omniboxInput?.select();
    event.preventDefault();
    return;
  }
  if (key === 'n' && event.shiftKey) {
    void commands.execute('windows.createPrivate');
    event.preventDefault();
    return;
  }
  if (key === 'n') {
    void commands.execute('windows.create');
    event.preventDefault();
    return;
  }
  if (key === 't' && event.shiftKey) {
    void tabs.reopenClosed();
    event.preventDefault();
    return;
  }
  if (key === 't') {
    void tabs.create();
    event.preventDefault();
    return;
  }
  if (key === 'f') {
    window.dispatchEvent(new CustomEvent('fubuki:show-find'));
    event.preventDefault();
    return;
  }
  if (event.key === '+' || event.key === '=') {
    void commands.execute('page.zoomIn');
    event.preventDefault();
    return;
  }
  if (event.key === '-') {
    void commands.execute('page.zoomOut');
    event.preventDefault();
    return;
  }
  if (event.key === '0') {
    void commands.execute('page.zoomReset');
    event.preventDefault();
    return;
  }
  if (key === 'b') {
    toggleSidebar();
    event.preventDefault();
    return;
  }
  if (event.key === ',') {
    navigateInternal('fubuki://settings/');
    event.preventDefault();
    return;
  }
  if (key === 'd') {
    void toggleBookmark();
    event.preventDefault();
    return;
  }
  if (!tab) return;
  if (key === 'w' && event.shiftKey) {
    void commands.execute('windows.close');
    event.preventDefault();
  } else if (key === 'w') {
    void tabs.close(tab.id);
    event.preventDefault();
  } else if (key === 'r') {
    void tabs.reload(tab.id);
    event.preventDefault();
  } else if (event.key === '[') {
    void tabs.goBack(tab.id);
    event.preventDefault();
  } else if (event.key === ']') {
    void tabs.goForward(tab.id);
    event.preventDefault();
  }
}

export default function App() {
  const [paletteOpen, setPaletteOpen] = createSignal(false);
  const [quietMode, setQuietMode] = createSignal(false);
  const [systemDark, setSystemDark] = createSignal(
    window.matchMedia?.('(prefers-color-scheme: dark)').matches ?? false,
  );

  createEffect(() => {
    const appearance = browserState.settings.appearance || 'system';
    const root = document.documentElement;

    root.dataset.theme =
      appearance === 'dark' || (appearance === 'system' && systemDark())
        ? 'dark'
        : 'light';

    root.dataset.sidebar =
      quietMode() || browserState.settings.sidebarVisible === 'hide'
        ? 'hide'
        : 'show';

    root.dataset.quietMode = quietMode() ? 'true' : 'false';
    root.lang = resolveLanguage(browserState.settings.language);
    root.dataset.language = browserState.settings.language || 'system';

    if (root.dataset.sidebarResizing !== 'true') {
      const width = clampSidebarWidth(
        Number(browserState.settings.sidebarWidth) || DEFAULT_SIDEBAR_WIDTH,
      );
      root.style.setProperty('--sidebar-width', `${width}px`);
    }
  });

  onMount(() => {
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    const onColorSchemeChange = (e: MediaQueryListEvent) =>
      setSystemDark(e.matches);
    mediaQuery.addEventListener('change', onColorSchemeChange);

    const disposeNativeEvents = bindNativeEvents();

    const onOpenPalette = () => setPaletteOpen(true);
    const onToggleSidebar = () => toggleSidebar();
    const onToggleActiveBookmark = () => void toggleBookmark();

    window.addEventListener('fubuki:open-palette', onOpenPalette);
    window.addEventListener('fubuki:toggle-sidebar', onToggleSidebar);
    window.addEventListener(
      'fubuki:toggle-active-bookmark',
      onToggleActiveBookmark,
    );
    window.addEventListener('keydown', onKeyDown);

    onCleanup(() => {
      mediaQuery.removeEventListener('change', onColorSchemeChange);
      disposeNativeEvents();
      window.removeEventListener('fubuki:open-palette', onOpenPalette);
      window.removeEventListener('fubuki:toggle-sidebar', onToggleSidebar);
      window.removeEventListener(
        'fubuki:toggle-active-bookmark',
        onToggleActiveBookmark,
      );
      window.removeEventListener('keydown', onKeyDown);
    });
  });

  return (
    <>
      <BrowserShell quietMode={quietMode()} />
      <CommandPalette
        open={paletteOpen()}
        quietMode={quietMode()}
        onClose={() => setPaletteOpen(false)}
        onToggleQuietMode={() => setQuietMode((value) => !value)}
      />
    </>
  );
}
