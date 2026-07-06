import { describe, it, expect, vi, beforeEach } from 'vitest';

// fubuki.ts は window に依存するため、リスナー管理ロジックを分離してテストする。
// 実装と同一の Map<eventName, Set<Listener>> パターンを使用。

type Listener = (payload: unknown) => void;

let listeners: Map<string, Set<Listener>>;

beforeEach(() => {
  listeners = new Map();
});

function emit(eventName: string, payload: unknown) {
  listeners.get(eventName)?.forEach((listener) => listener(payload));
}

function on(eventName: string, listener: Listener): () => void {
  const set = listeners.get(eventName) ?? new Set<Listener>();
  set.add(listener);
  listeners.set(eventName, set);
  return () => set.delete(listener);
}

describe('event listener management', () => {
  // --- 基本動作 ---

  it('calls listener when event is emitted', () => {
    const spy = vi.fn();
    on('test:event', spy);

    emit('test:event', { data: 42 });

    expect(spy).toHaveBeenCalledOnce();
    expect(spy).toHaveBeenCalledWith({ data: 42 });
  });

  it('passes string payload correctly', () => {
    const spy = vi.fn();
    on('test:event', spy);

    emit('test:event', 'hello');

    expect(spy).toHaveBeenCalledWith('hello');
  });

  it('passes number payload correctly', () => {
    const spy = vi.fn();
    on('test:event', spy);

    emit('test:event', 123);

    expect(spy).toHaveBeenCalledWith(123);
  });

  it('passes boolean payload correctly', () => {
    const spy = vi.fn();
    on('test:event', spy);

    emit('test:event', true);

    expect(spy).toHaveBeenCalledWith(true);
  });

  it('passes null payload correctly', () => {
    const spy = vi.fn();
    on('test:event', spy);

    emit('test:event', null);

    expect(spy).toHaveBeenCalledWith(null);
  });

  it('passes undefined payload correctly', () => {
    const spy = vi.fn();
    on('test:event', spy);

    emit('test:event', undefined);

    expect(spy).toHaveBeenCalledWith(undefined);
  });

  it('passes complex object payload correctly', () => {
    const spy = vi.fn();
    on('test:event', spy);

    const payload = {
      tabs: [{ id: 'tab-1', title: 'Test' }],
      activeTabId: 'tab-1',
    };
    emit('test:event', payload);

    expect(spy).toHaveBeenCalledWith(payload);
  });

  it('passes array payload correctly', () => {
    const spy = vi.fn();
    on('test:event', spy);

    emit('test:event', [1, 'two', { three: 3 }]);

    expect(spy).toHaveBeenCalledWith([1, 'two', { three: 3 }]);
  });

  // --- dispose 機能 ---

  it('returns a dispose function that removes the listener', () => {
    const spy = vi.fn();
    const dispose = on('test:event', spy);

    dispose();
    emit('test:event', null);

    expect(spy).not.toHaveBeenCalled();
  });

  it('dispose is idempotent (calling twice does not throw)', () => {
    const spy = vi.fn();
    const dispose = on('test:event', spy);

    dispose();
    expect(() => dispose()).not.toThrow();
    expect(spy).not.toHaveBeenCalled();
  });

  it('dispose returns true on first call, false on subsequent calls', () => {
    const spy = vi.fn();
    const dispose = on('test:event', spy);

    expect(dispose()).toBe(true);
    expect(dispose()).toBe(false);
  });

  it('re-registering after dispose works correctly', () => {
    const spy1 = vi.fn();
    const spy2 = vi.fn();
    const dispose1 = on('test:event', spy1);

    dispose1();
    on('test:event', spy2);

    emit('test:event', 'after-re-register');

    expect(spy1).not.toHaveBeenCalled();
    expect(spy2).toHaveBeenCalledOnce();
    expect(spy2).toHaveBeenCalledWith('after-re-register');
  });

  // --- 複数リスナー ---

  it('supports multiple listeners on the same event', () => {
    const spy1 = vi.fn();
    const spy2 = vi.fn();
    on('test:event', spy1);
    on('test:event', spy2);

    emit('test:event', 'hello');

    expect(spy1).toHaveBeenCalledWith('hello');
    expect(spy2).toHaveBeenCalledWith('hello');
  });

  it('calls multiple listeners in registration order', () => {
    const order: number[] = [];
    on('test:event', () => order.push(1));
    on('test:event', () => order.push(2));
    on('test:event', () => order.push(3));

    emit('test:event', null);

    expect(order).toEqual([1, 2, 3]);
  });

  it('dispose only removes its own listener', () => {
    const spy1 = vi.fn();
    const spy2 = vi.fn();
    const dispose1 = on('test:event', spy1);
    on('test:event', spy2);

    dispose1();
    emit('test:event', null);

    expect(spy1).not.toHaveBeenCalled();
    expect(spy2).toHaveBeenCalledOnce();
  });

  it('deduplicates same listener function (Set behavior)', () => {
    const spy = vi.fn();
    on('test:event', spy);
    on('test:event', spy); // 同じ関数を 2 回登録

    emit('test:event', 'dedup-test');

    // Set なので 1 回だけ呼ばれる
    expect(spy).toHaveBeenCalledTimes(1);
  });

  it('handles many listeners on the same event', () => {
    const spies = Array.from({ length: 50 }, () => vi.fn());
    spies.forEach((spy) => on('test:event', spy));

    emit('test:event', 'many');

    spies.forEach((spy) => {
      expect(spy).toHaveBeenCalledOnce();
      expect(spy).toHaveBeenCalledWith('many');
    });
  });

  // --- イベント分離 ---

  it('does not cross-emit between different events', () => {
    const spyA = vi.fn();
    const spyB = vi.fn();
    on('event:a', spyA);
    on('event:b', spyB);

    emit('event:a', null);

    expect(spyA).toHaveBeenCalledOnce();
    expect(spyB).not.toHaveBeenCalled();
  });

  it('emitting unregistered event does not throw', () => {
    expect(() => emit('nonexistent:event', null)).not.toThrow();
  });

  it('same listener on different events receives only its own events', () => {
    const spy = vi.fn();
    on('event:a', spy);
    on('event:b', spy);

    emit('event:a', 'from-a');

    expect(spy).toHaveBeenCalledTimes(1);
    expect(spy).toHaveBeenCalledWith('from-a');
  });

  it('dispose on one event does not affect other events', () => {
    const spyA = vi.fn();
    const spyB = vi.fn();
    const disposeA = on('event:a', spyA);
    on('event:b', spyB);

    disposeA();
    emit('event:a', null);
    emit('event:b', 'still-works');

    expect(spyA).not.toHaveBeenCalled();
    expect(spyB).toHaveBeenCalledOnce();
    expect(spyB).toHaveBeenCalledWith('still-works');
  });

  // --- emit 呼び出し回数 ---

  it('emitting multiple times calls listener each time', () => {
    const spy = vi.fn();
    on('test:event', spy);

    emit('test:event', 'first');
    emit('test:event', 'second');
    emit('test:event', 'third');

    expect(spy).toHaveBeenCalledTimes(3);
    expect(spy).toHaveBeenNthCalledWith(1, 'first');
    expect(spy).toHaveBeenNthCalledWith(2, 'second');
    expect(spy).toHaveBeenNthCalledWith(3, 'third');
  });

  it('listener receives only events after registration', () => {
    emit('test:event', 'before');

    const spy = vi.fn();
    on('test:event', spy);

    emit('test:event', 'after');

    expect(spy).toHaveBeenCalledTimes(1);
    expect(spy).toHaveBeenCalledWith('after');
  });
});
