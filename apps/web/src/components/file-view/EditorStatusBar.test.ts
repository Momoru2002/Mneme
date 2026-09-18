import { describe, expect, it } from 'vitest';
import { saveStatusLabel } from './EditorStatusBar.tsx';

describe('saveStatusLabel', () => {
  it('maps each save status kind to its text', () => {
    expect(saveStatusLabel({ kind: 'saving' })).toBe('Saving…');
    expect(saveStatusLabel({ kind: 'dirty' })).toBe('Unsaved changes…');
    expect(saveStatusLabel({ kind: 'saved', at: 0 })).toBe('Saved');
    expect(saveStatusLabel({ kind: 'conflict' })).toBe('Conflict');
    expect(saveStatusLabel({ kind: 'error', message: 'boom' })).toBe('Save failed');
    expect(saveStatusLabel({ kind: 'idle' })).toBe('All changes saved');
  });
});
