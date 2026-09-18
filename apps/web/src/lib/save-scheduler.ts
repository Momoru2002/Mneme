/**
 * Decouples autosave timing from React so the data-loss paths are unit-testable.
 * The EDITING-3 bug was that the editor's cleanup `clearTimeout`-ed the debounce
 * but never flushed it — switching files within the debounce window dropped the
 * last edit. Here `flush()` is the explicit, awaited save used on file-switch,
 * window blur, and app close.
 */
export type SaveScheduler = {
  /** Buffer `content` and (re)arm the debounce timer. */
  schedule(content: string): void;
  /** Cancel the timer and save any pending content immediately. */
  flush(): Promise<void>;
  /** Cancel the timer and DROP pending content (explicit discard). */
  cancel(): void;
  /** Is there unsaved buffered content? */
  isDirty(): boolean;
};

export function createSaveScheduler(
  save: (content: string) => Promise<void>,
  delayMs: number,
): SaveScheduler {
  let timer: ReturnType<typeof setTimeout> | null = null;
  let pending: string | null = null;

  const clearTimer = (): void => {
    if (timer !== null) {
      clearTimeout(timer);
      timer = null;
    }
  };

  const flush = async (): Promise<void> => {
    clearTimer();
    if (pending === null) return;
    const content = pending;
    pending = null;
    // If `save` rejects, the error bubbles to the caller; `pending` is already
    // cleared (isDirty() is false), so this scheduler does NOT retry — the caller
    // is responsible for surfacing the failure and retaining the buffer.
    await save(content);
  };

  return {
    schedule(content: string): void {
      pending = content;
      clearTimer();
      timer = setTimeout(() => {
        void flush();
      }, delayMs);
    },
    flush,
    cancel(): void {
      clearTimer();
      pending = null;
    },
    isDirty(): boolean {
      return pending !== null;
    },
  };
}
