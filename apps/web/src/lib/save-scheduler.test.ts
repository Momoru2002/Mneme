import { afterEach, describe, expect, it, vi } from 'vitest';
import { createSaveScheduler } from './save-scheduler.ts';

afterEach(() => {
  vi.useRealTimers();
});

describe('createSaveScheduler', () => {
  it('flush() persists the pending content — EDITING-3 regression (switch files mid-autosave)', async () => {
    const save = vi.fn().mockResolvedValue(undefined);
    const s = createSaveScheduler(save, 2000);

    s.schedule('hello'); // user typed, debounce not yet elapsed
    expect(s.isDirty()).toBe(true);

    await s.flush(); // simulates switching files / blur / close

    expect(save).toHaveBeenCalledTimes(1);
    expect(save).toHaveBeenCalledWith('hello');
    expect(s.isDirty()).toBe(false);
  });

  it('the debounce timer eventually saves on its own', async () => {
    vi.useFakeTimers();
    const save = vi.fn().mockResolvedValue(undefined);
    const s = createSaveScheduler(save, 2000);

    s.schedule('typed');
    expect(save).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(2000);

    expect(save).toHaveBeenCalledTimes(1);
    expect(save).toHaveBeenCalledWith('typed');
    expect(s.isDirty()).toBe(false);
  });

  it('scheduling again before flush replaces the pending content', async () => {
    const save = vi.fn().mockResolvedValue(undefined);
    const s = createSaveScheduler(save, 2000);
    s.schedule('first');
    s.schedule('second');
    await s.flush();
    expect(save).toHaveBeenCalledTimes(1);
    expect(save).toHaveBeenCalledWith('second');
  });

  it('cancel() discards pending without saving (explicit discard path)', async () => {
    const save = vi.fn().mockResolvedValue(undefined);
    const s = createSaveScheduler(save, 2000);
    s.schedule('dropme');
    s.cancel();
    await s.flush();
    expect(save).not.toHaveBeenCalled();
    expect(s.isDirty()).toBe(false);
  });

  it('flush() with nothing pending is a no-op', async () => {
    const save = vi.fn().mockResolvedValue(undefined);
    const s = createSaveScheduler(save, 2000);
    await s.flush();
    expect(save).not.toHaveBeenCalled();
  });
});
