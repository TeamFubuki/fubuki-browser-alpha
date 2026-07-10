import { describe, expect, it, vi } from 'vitest';
import { createSidebarWidthSync } from '../sidebarWidthSync';

describe('createSidebarWidthSync', () => {
  it('coalesces queued updates and preserves the final width', async () => {
    let releaseFirst: (() => void) | undefined;
    const sent: number[] = [];
    const sender = vi.fn(async (width: number) => {
      sent.push(width);
      if (sent.length === 1) {
        await new Promise<void>((resolve) => {
          releaseFirst = resolve;
        });
      }
    });
    const sync = createSidebarWidthSync(sender);

    sync.update(180);
    sync.update(190);
    const flushed = sync.flush(220);
    releaseFirst?.();
    await flushed;

    expect(sent).toEqual([180, 220]);
  });

  it('continues after a transient bridge failure', async () => {
    const sender = vi
      .fn<(width: number) => Promise<void>>()
      .mockRejectedValueOnce(new Error('bridge unavailable'))
      .mockResolvedValue(undefined);
    const sync = createSidebarWidthSync(sender);

    sync.update(180);
    await sync.flush(240);

    expect(sender).toHaveBeenCalledTimes(2);
    expect(sender).toHaveBeenLastCalledWith(240);
  });

  it('ignores updates after disposal', async () => {
    const sender = vi.fn(async () => undefined);
    const sync = createSidebarWidthSync(sender);
    sync.dispose();

    sync.update(240);
    await sync.flush(250);

    expect(sender).not.toHaveBeenCalled();
  });
});
