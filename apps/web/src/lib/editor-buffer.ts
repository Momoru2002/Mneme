const CURSOR_MARKER = '{{cursor}}';

/**
 * Re-join a file's original frontmatter prefix with an edited body. The editor
 * only edits the body; the frontmatter prefix is everything in `original` that
 * precedes `originalBody`. Mirrors the logic previously inlined in
 * EditorWorkbench so it can be tested in isolation.
 */
export function reconstructContent(
  original: string,
  originalBody: string,
  newBody: string,
): string {
  // lastIndexOf (not indexOf): the body is always the LAST occurrence in a
  // well-formed file, so this is robust to the body text also appearing as a
  // YAML value inside the frontmatter. Callers must pass `originalBody` and
  // `original` from the SAME FileContent response (body is a suffix of content).
  const bodyStart = originalBody ? original.lastIndexOf(originalBody) : original.length;
  if (bodyStart === -1) {
    // Body not found in original — arguments drifted (a caller bug). We cannot
    // locate the frontmatter split, so fall back to the new body alone rather
    // than guess. Made explicit + tested so the failure mode is documented.
    return newBody;
  }
  const prefix = bodyStart > 0 ? original.slice(0, bodyStart) : '';
  return `${prefix}${newBody}`;
}

/**
 * Insert `rendered` over the [from, to) selection. If the template contains a
 * `{{cursor}}` marker, the marker is removed and the returned `cursor` points to
 * where it was (so callers can place the caret there). Without a marker, the
 * caret lands at the end of the inserted text. Only the FIRST marker is honored.
 */
export function insertTemplateAtCursor(
  doc: string,
  from: number,
  to: number,
  rendered: string,
): { doc: string; cursor: number } {
  const markerIdx = rendered.indexOf(CURSOR_MARKER);
  const insert = markerIdx === -1 ? rendered : rendered.replace(CURSOR_MARKER, '');
  const caretOffset = markerIdx === -1 ? insert.length : markerIdx;
  const next = doc.slice(0, from) + insert + doc.slice(to);
  return { doc: next, cursor: from + caretOffset };
}

/**
 * Decide whether to overwrite the local editor buffer with freshly-fetched
 * server content. The STATES-1/EDITING-3 rule: NEVER clobber unsaved edits;
 * otherwise prime only when the server content actually changed.
 */
export function shouldPrimeBuffer(args: {
  isDirty: boolean;
  serverHash: string;
  lastSavedHash: string | null;
}): boolean {
  if (args.isDirty) return false;
  return args.lastSavedHash !== args.serverHash;
}
