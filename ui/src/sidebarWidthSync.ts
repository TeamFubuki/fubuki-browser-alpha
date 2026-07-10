export type SidebarWidthSender = (width: number) => Promise<unknown>;

/**
 * Called after a native update completes successfully.
 * Never called after `dispose()` has been invoked.
 */
export type SidebarWidthCallback = (width: number) => void;

export interface SidebarWidthSyncOptions {
  send: SidebarWidthSender;
  /** Called after each successful native update. Not called after dispose. */
  onApplied?: SidebarWidthCallback;
}

/**
 * Coalesces fast pointer updates so the bridge only receives the latest queued
 * width. CSS is applied *after* the native update completes via `onApplied`.
 */
export function createSidebarWidthSync(
  optionsOrSend: SidebarWidthSender | SidebarWidthSyncOptions,
) {
  const options =
    typeof optionsOrSend === 'function'
      ? { send: optionsOrSend }
      : optionsOrSend;
  const { send, onApplied } = options;

  let queuedWidth: number | undefined;
  let running: Promise<void> | undefined;
  let disposed = false;

  const start = () => {
    if (running || disposed) return;

    running = (async () => {
      while (!disposed && queuedWidth !== undefined) {
        const width = queuedWidth;
        queuedWidth = undefined;
        try {
          await send(width);
          if (!disposed) onApplied?.(width);
        } catch (error) {
          console.error('[Fubuki] Failed to resize native sidebar:', error);
        }
      }
    })().finally(() => {
      running = undefined;
      // A new update may have been queued while we were running.
      if (!disposed && queuedWidth !== undefined) start();
    });
  };

  const update = (width: number) => {
    if (disposed) return;
    queuedWidth = width;
    start();
  };

  const flush = async (width: number) => {
    update(width);
    // Wait until the loop fully drains: running must be undefined AND
    // queuedWidth must be consumed. The finally block in `start` may
    // re-arm the loop if a new update arrived while we were waiting, so
    // we keep polling until both conditions are met.
    for (;;) {
      const p = running;
      if (p) await p;
      if (queuedWidth === undefined && running === undefined) break;
    }
  };

  const dispose = () => {
    disposed = true;
    queuedWidth = undefined;
    running = undefined;
  };

  return { update, flush, dispose };
}
