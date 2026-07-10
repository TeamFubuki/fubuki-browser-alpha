export type SidebarWidthSender = (width: number) => Promise<unknown>;

/** Coalesces fast pointer updates so the CEF bridge only receives the latest queued width. */
export function createSidebarWidthSync(send: SidebarWidthSender) {
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
        } catch (error) {
          console.error('[Fubuki] Failed to resize native sidebar:', error);
        }
      }
    })().finally(() => {
      running = undefined;
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
    while (running) await running;
  };

  const dispose = () => {
    disposed = true;
    queuedWidth = undefined;
  };

  return { update, flush, dispose };
}
