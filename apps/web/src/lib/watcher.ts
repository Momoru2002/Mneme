/**
 * Real-time well change notifications.
 *
 * Slice 6 delivered these over SSE (`EventSource('/api/watch')`) through the
 * Fastify sidecar. Slice 7 removed the sidecar, so that transport is gone. The
 * native replacement is a Rust fs-watcher that emits a Tauri event
 * (`app.emit('well.changed', …)`) which this hook would subscribe to via
 * `listen` from `@tauri-apps/api/event`.
 *
 * That emitter is **P4**, deferred to slice 8 (see the slice-7 run-board). Until
 * it lands this is an intentional, documented no-op — the tree stays fresh via
 * react-query `staleTime` + explicit cache invalidation on every mutation, not
 * by an external-change push. This hook is kept as the single integration seam
 * so slice 8 only has to fill in the `listen` subscription here. The parameter
 * is `_`-prefixed (intentionally unused) until then.
 */
export function useFileWatcher(_wellId: string | null | undefined): void {
  // No-op until slice-8 native Tauri events (P4).
}
