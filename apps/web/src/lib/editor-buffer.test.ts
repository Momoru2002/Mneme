import { describe, expect, it } from 'vitest';
import { insertTemplateAtCursor, reconstructContent, shouldPrimeBuffer } from './editor-buffer.ts';

describe('reconstructContent', () => {
  it('re-joins the original frontmatter prefix with the new body', () => {
    const original = '---\ntitle: X\n---\nold body';
    const body = 'old body';
    expect(reconstructContent(original, body, 'new body')).toBe('---\ntitle: X\n---\nnew body');
  });

  it('returns the new body verbatim when there is no frontmatter', () => {
    expect(reconstructContent('just body', 'just body', 'edited')).toBe('edited');
  });

  it('handles an empty original body (frontmatter-only file)', () => {
    const original = '---\ntitle: X\n---\n';
    expect(reconstructContent(original, '', 'typed')).toBe('---\ntitle: X\n---\ntyped');
  });

  it('falls back to the new body when the original body is not found (arg drift)', () => {
    expect(reconstructContent('---\ntitle: X\n---\ndisk body', 'stale body', 'new body')).toBe(
      'new body',
    );
  });
});

describe('insertTemplateAtCursor', () => {
  it('inserts at the selection and reports the caret after the insert', () => {
    const r = insertTemplateAtCursor('abXYcd', 2, 4, 'Z'); // replace "XY" with "Z"
    expect(r.doc).toBe('abZcd');
    expect(r.cursor).toBe(3); // end of inserted text
  });

  it('honors {{cursor}} by placing the caret where the marker was', () => {
    const r = insertTemplateAtCursor('start\n', 6, 6, 'before{{cursor}}after');
    expect(r.doc).toBe('start\nbeforeafter');
    expect(r.cursor).toBe('start\nbefore'.length); // caret sits between before|after
  });

  it('strips only the first {{cursor}} marker (leaves the rest as text)', () => {
    const r = insertTemplateAtCursor('', 0, 0, 'a{{cursor}}b{{cursor}}c');
    expect(r.doc).toBe('ab{{cursor}}c');
    expect(r.cursor).toBe(1);
  });

  it('combines a non-empty selection with a {{cursor}} marker', () => {
    const r = insertTemplateAtCursor('abXYcd', 2, 4, 'Z{{cursor}}W'); // replace "XY"
    expect(r.doc).toBe('abZWcd');
    expect(r.cursor).toBe(3); // caret between Z and W
  });
});

describe('shouldPrimeBuffer', () => {
  it('never overwrites a dirty buffer — STATES-1/EDITING-3 anti-clobber', () => {
    expect(shouldPrimeBuffer({ isDirty: true, serverHash: 'b', lastSavedHash: 'a' })).toBe(false);
  });

  it('primes when clean and the server hash differs from what we last saved', () => {
    expect(shouldPrimeBuffer({ isDirty: false, serverHash: 'b', lastSavedHash: 'a' })).toBe(true);
  });

  it('does not re-prime when clean and the hash is unchanged', () => {
    expect(shouldPrimeBuffer({ isDirty: false, serverHash: 'a', lastSavedHash: 'a' })).toBe(false);
  });

  it('primes a never-loaded buffer (null lastSavedHash)', () => {
    expect(shouldPrimeBuffer({ isDirty: false, serverHash: 'a', lastSavedHash: null })).toBe(true);
  });
});
