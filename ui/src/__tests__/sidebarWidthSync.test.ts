import { describe, expect, it, vi } from 'vitest';
import { createSidebarWidthSync } from '../sidebarWidthSync';
import {
  clampSidebarWidth,
  MIN_SIDEBAR_WIDTH,
  MAX_SIDEBAR_WIDTH,
} from '../sidebarSizing';

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

  it('calls onApplied after each native update completes', async () => {
    const applied: number[] = [];
    const sender = vi.fn(async () => undefined);
    const sync = createSidebarWidthSync({
      send: sender,
      onApplied: (width) => applied.push(width),
    });

    sync.update(180);
    await sync.flush(200);

    expect(sender).toHaveBeenCalledTimes(2);
    expect(applied).toEqual([180, 200]);
  });

  it('does not call onApplied before native update completes', async () => {
    let releaseFirst: (() => void) | undefined;
    const applied: number[] = [];
    const sender = vi.fn(async (width: number) => {
      if (width === 180) {
        await new Promise<void>((resolve) => {
          releaseFirst = resolve;
        });
      }
    });
    const sync = createSidebarWidthSync({
      send: sender,
      onApplied: (width) => applied.push(width),
    });

    sync.update(180);
    await new Promise((r) => setTimeout(r, 10));
    expect(applied).toEqual([]);

    releaseFirst?.();
    await sync.flush(200);
    expect(applied).toEqual([180, 200]);
  });

  it('calls onApplied for each completed native update', async () => {
    const applied: number[] = [];
    const sender = vi.fn(async () => undefined);
    const sync = createSidebarWidthSync({
      send: sender,
      onApplied: (width) => applied.push(width),
    });

    sync.update(180);
    await sync.flush(200);

    sync.update(220);
    await sync.flush(240);

    expect(applied).toEqual([180, 200, 220, 240]);
  });

  it('coalesces rapid updates and only sends queued + flushed widths', async () => {
    const sent: number[] = [];
    const applied: number[] = [];
    const sender = vi.fn(async (width: number) => {
      sent.push(width);
    });
    const sync = createSidebarWidthSync({
      send: sender,
      onApplied: (width) => applied.push(width),
    });

    sync.update(180);
    sync.update(190);
    sync.update(200);
    sync.update(210);
    await sync.flush(220);

    expect(sent).toEqual([180, 220]);
    expect(applied).toEqual([180, 220]);
  });

  it('flush waits for in-flight native update before returning', async () => {
    let resolveFirst: (() => void) | undefined;
    const sent: number[] = [];
    const sender = vi.fn(async (width: number) => {
      sent.push(width);
      if (sent.length === 1) {
        await new Promise<void>((resolve) => {
          resolveFirst = resolve;
        });
      }
    });
    const sync = createSidebarWidthSync(sender);

    sync.update(180);
    const flushPromise = sync.flush(220);
    let resolved = false;
    flushPromise.then(() => {
      resolved = true;
    });
    await new Promise((r) => setTimeout(r, 10));
    expect(resolved).toBe(false);

    resolveFirst?.();
    await flushPromise;
    expect(resolved).toBe(true);
    expect(sent).toEqual([180, 220]);
  });

  it('onApplied is not called when native update fails', async () => {
    const applied: number[] = [];
    const sender = vi
      .fn<(width: number) => Promise<void>>()
      .mockRejectedValueOnce(new Error('bridge error'))
      .mockResolvedValue(undefined);
    const sync = createSidebarWidthSync({
      send: sender,
      onApplied: (width) => applied.push(width),
    });

    sync.update(180);
    await sync.flush(200);

    expect(applied).toEqual([200]);
  });

  it('dispose prevents onApplied from being called', async () => {
    let releaseFirst: (() => void) | undefined;
    const applied: number[] = [];
    const sender = vi.fn(async (width: number) => {
      if (width === 180) {
        await new Promise<void>((resolve) => {
          releaseFirst = resolve;
        });
      }
    });
    const sync = createSidebarWidthSync({
      send: sender,
      onApplied: (width) => applied.push(width),
    });

    sync.update(180);
    sync.dispose();
    releaseFirst?.();
    await new Promise((r) => setTimeout(r, 10));

    expect(applied).toEqual([]);
  });
});

describe('sidebar width constraints', () => {
  it('MIN <= DEFAULT <= MAX', () => {
    expect(MIN_SIDEBAR_WIDTH).toBeLessThanOrEqual(MAX_SIDEBAR_WIDTH);
  });

  it('clampSidebarWidth respects minimum', () => {
    expect(clampSidebarWidth(MIN_SIDEBAR_WIDTH - 10)).toBe(MIN_SIDEBAR_WIDTH);
    expect(clampSidebarWidth(0)).toBe(MIN_SIDEBAR_WIDTH);
    expect(clampSidebarWidth(-100)).toBe(MIN_SIDEBAR_WIDTH);
  });

  it('clampSidebarWidth respects maximum', () => {
    expect(clampSidebarWidth(MAX_SIDEBAR_WIDTH + 10)).toBe(MAX_SIDEBAR_WIDTH);
    expect(clampSidebarWidth(500)).toBe(MAX_SIDEBAR_WIDTH);
  });

  it('clampSidebarWidth allows values within range', () => {
    const validWidths = [170, 180, 196, 220, 250, 280];
    for (const width of validWidths) {
      expect(clampSidebarWidth(width)).toBe(width);
    }
  });

  it('clampSidebarWidth rounds fractional values', () => {
    expect(clampSidebarWidth(195.4)).toBe(195);
    expect(clampSidebarWidth(195.6)).toBe(196);
  });

  it('clampSidebarWidth returns DEFAULT for invalid input', () => {
    expect(clampSidebarWidth(NaN)).toBe(196);
    expect(clampSidebarWidth(Infinity)).toBe(196);
    expect(clampSidebarWidth(-Infinity)).toBe(196);
  });
});
