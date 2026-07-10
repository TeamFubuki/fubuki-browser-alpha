import { createSignal, onCleanup } from 'solid-js';
import { fubuki } from '../bridge/fubuki';
import { browserState, setBrowserState } from '../stores/browserStore';
import { clampSidebarWidth, DEFAULT_SIDEBAR_WIDTH } from '../sidebarSizing';
import { createSidebarWidthSync } from '../sidebarWidthSync';

function applyCssSidebarWidth(width: number) {
  document.documentElement.style.setProperty('--sidebar-width', `${width}px`);
}

function currentSidebarWidth() {
  const cssWidth = Number.parseFloat(
    getComputedStyle(document.documentElement).getPropertyValue(
      '--sidebar-width',
    ),
  );
  if (Number.isFinite(cssWidth)) {
    return clampSidebarWidth(cssWidth);
  }
  return clampSidebarWidth(
    Number(browserState.settings.sidebarWidth) || DEFAULT_SIDEBAR_WIDTH,
  );
}

export function useSidebarResize() {
  const [resizing, setResizing] = createSignal(false);
  let handle: HTMLElement | undefined;
  let activePointerId = -1;
  let active = false;
  let startX = 0;
  let startWidth = DEFAULT_SIDEBAR_WIDTH;
  let pendingWidth = DEFAULT_SIDEBAR_WIDTH;
  let nativeFrame = 0;
  const nativeWidth = createSidebarWidthSync({
    send: (width) => fubuki.invoke('ui.setSidebarWidth', { width }),
    onApplied: (width) => applyCssSidebarWidth(width),
  });

  const saveWidth = async (width: number) => {
    await nativeWidth.flush(width);
    await fubuki.invoke('settings.set', {
      key: 'sidebarWidth',
      value: String(width),
    });
  };

  const removeListeners = () => {
    if (handle) {
      handle.removeEventListener('pointermove', onPointerMove);
      handle.removeEventListener('pointerup', onPointerUp);
      handle.removeEventListener('pointercancel', onPointerCancel);
      handle.removeEventListener('lostpointercapture', onLostPointerCapture);
    }
  };

  const finishResize = (clientX?: number) => {
    if (!active) return;

    active = false;

    const width = clampSidebarWidth(
      typeof clientX === 'number'
        ? startWidth + clientX - startX
        : pendingWidth,
    );
    pendingWidth = width;

    if (nativeFrame) {
      cancelAnimationFrame(nativeFrame);
      nativeFrame = 0;
    }

    removeListeners();
    delete document.documentElement.dataset.sidebarResizing;
    setResizing(false);

    if (handle?.hasPointerCapture(activePointerId)) {
      handle.releasePointerCapture(activePointerId);
    }
    activePointerId = -1;

    void saveWidth(width).catch((error) => {
      console.error('[Fubuki] Failed to save sidebar width:', error);
      applyCssSidebarWidth(startWidth);
      setBrowserState('settings', 'sidebarWidth', String(startWidth));
      setBrowserState('status', 'Error');
      void nativeWidth.flush(startWidth);
    });
  };

  const onPointerMove = (event: PointerEvent) => {
    if (!active || event.pointerId !== activePointerId) return;

    pendingWidth = clampSidebarWidth(startWidth + event.clientX - startX);
    applyCssSidebarWidth(pendingWidth);
    if (!nativeFrame) {
      nativeFrame = requestAnimationFrame(() => {
        nativeFrame = 0;
        nativeWidth.update(pendingWidth);
      });
    }
  };

  const onPointerUp = (event: PointerEvent) => {
    if (event.pointerId !== activePointerId) return;
    finishResize(event.clientX);
  };

  const onPointerCancel = (event: PointerEvent) => {
    if (event.pointerId !== activePointerId) return;
    finishResize();
  };

  const onLostPointerCapture = (event: PointerEvent) => {
    if (event.pointerId !== activePointerId) return;
    finishResize();
  };

  const startResize = (
    event: PointerEvent & { currentTarget: HTMLElement },
  ) => {
    if (event.button !== 0 || active) return;
    event.preventDefault();

    handle = event.currentTarget;
    activePointerId = event.pointerId;
    startX = event.clientX;
    startWidth = currentSidebarWidth();
    pendingWidth = startWidth;
    active = true;

    document.documentElement.dataset.sidebarResizing = 'true';
    setResizing(true);
    handle.setPointerCapture(activePointerId);
    handle.addEventListener('pointermove', onPointerMove);
    handle.addEventListener('pointerup', onPointerUp);
    handle.addEventListener('pointercancel', onPointerCancel);
    handle.addEventListener('lostpointercapture', onLostPointerCapture);
  };

  const resetWidth = () => {
    const width = clampSidebarWidth(DEFAULT_SIDEBAR_WIDTH);
    void saveWidth(width).catch((error) =>
      console.error('[Fubuki] Failed to reset sidebar width:', error),
    );
  };

  onCleanup(() => {
    removeListeners();
    if (active && handle?.hasPointerCapture(activePointerId)) {
      handle.releasePointerCapture(activePointerId);
    }
    active = false;
    activePointerId = -1;
    if (nativeFrame) cancelAnimationFrame(nativeFrame);
    nativeWidth.dispose();
    delete document.documentElement.dataset.sidebarResizing;
  });

  return { resizing, startResize, resetWidth };
}
