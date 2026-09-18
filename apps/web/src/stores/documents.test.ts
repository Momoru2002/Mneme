import { beforeEach, describe, expect, it, vi } from 'vitest';
import { docKeyOf, registerScheduler, schedulerOf, useDocumentsStore } from './documents.ts';

beforeEach(() => {
  useDocumentsStore.setState({ docs: {} });
});

describe('documents registry', () => {
  it('docKeyOf joins well + path', () => {
    expect(docKeyOf('w1', 'a/b.md')).toBe('w1::a/b.md');
  });

  it('acquire creates an entry with refCount 1; second acquire bumps to 2', () => {
    const key = docKeyOf('w1', 'a.md');
    useDocumentsStore.getState().acquire('w1', 'a.md');
    expect(useDocumentsStore.getState().docs[key]?.refCount).toBe(1);
    useDocumentsStore.getState().acquire('w1', 'a.md');
    expect(useDocumentsStore.getState().docs[key]?.refCount).toBe(2);
  });

  it('release decrements; reaching 0 flushes the scheduler and deletes the entry', async () => {
    const key = docKeyOf('w1', 'a.md');
    useDocumentsStore.getState().acquire('w1', 'a.md');
    const flush = vi.fn().mockResolvedValue(undefined);
    registerScheduler(key, {
      schedule: vi.fn(),
      flush,
      cancel: vi.fn(),
      isDirty: () => false,
    });
    await useDocumentsStore.getState().release('w1', 'a.md');
    expect(flush).toHaveBeenCalledOnce();
    expect(useDocumentsStore.getState().docs[key]).toBeUndefined();
    expect(schedulerOf(key)).toBeUndefined();
  });

  it('release on a multiply-acquired doc does not flush until the last release', async () => {
    const key = docKeyOf('w1', 'a.md');
    useDocumentsStore.getState().acquire('w1', 'a.md');
    useDocumentsStore.getState().acquire('w1', 'a.md');
    const flush = vi.fn().mockResolvedValue(undefined);
    registerScheduler(key, { schedule: vi.fn(), flush, cancel: vi.fn(), isDirty: () => false });
    await useDocumentsStore.getState().release('w1', 'a.md');
    expect(flush).not.toHaveBeenCalled();
    expect(useDocumentsStore.getState().docs[key]?.refCount).toBe(1);
    await useDocumentsStore.getState().release('w1', 'a.md');
    expect(flush).toHaveBeenCalledOnce();
  });

  it('patchView merges reactive view fields', () => {
    const key = docKeyOf('w1', 'a.md');
    useDocumentsStore.getState().acquire('w1', 'a.md');
    useDocumentsStore.getState().patchView(key, { buffer: 'hello', dirty: true });
    expect(useDocumentsStore.getState().docs[key]?.buffer).toBe('hello');
    expect(useDocumentsStore.getState().docs[key]?.dirty).toBe(true);
  });

  it('patchView before acquire still records the projection (split-pane blank-buffer regression)', () => {
    // Repro of the second-split-pane blank bug: the host primes the buffer before
    // EditorWorkbench's acquire runs (child effects fire before parent effects).
    // The projection must survive, and the later acquire must transition 0→1.
    const key = docKeyOf('w1', 'a.md');
    useDocumentsStore.getState().patchView(key, { buffer: 'hello', ready: true });
    expect(useDocumentsStore.getState().docs[key]?.buffer).toBe('hello');
    expect(useDocumentsStore.getState().docs[key]?.refCount).toBe(0);
    useDocumentsStore.getState().acquire('w1', 'a.md');
    const doc = useDocumentsStore.getState().docs[key];
    expect(doc?.refCount).toBe(1);
    expect(doc?.buffer).toBe('hello'); // not clobbered back to ''
  });

  it('re-acquire during flush does not delete the doc entry', async () => {
    const key = docKeyOf('w1', 'a.md');
    useDocumentsStore.getState().acquire('w1', 'a.md');
    let resolveFlush!: () => void;
    const flush = vi.fn().mockReturnValue(
      new Promise<void>((r) => {
        resolveFlush = r;
      }),
    );
    registerScheduler(key, { schedule: vi.fn(), flush, cancel: vi.fn(), isDirty: () => false });
    const releasePromise = useDocumentsStore.getState().release('w1', 'a.md');
    // Re-acquire while flush is pending (user reopens the file)
    useDocumentsStore.getState().acquire('w1', 'a.md');
    resolveFlush();
    await releasePromise;
    expect(useDocumentsStore.getState().docs[key]).toBeDefined();
    expect(useDocumentsStore.getState().docs[key]?.refCount).toBe(1);
  });
});
