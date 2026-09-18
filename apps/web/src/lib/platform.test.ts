import { describe, expect, it } from 'vitest';
import { keyGlyph } from './platform.ts';

describe('keyGlyph', () => {
  it('maps modifier tokens to mac glyphs', () => {
    expect(keyGlyph('Mod', true)).toBe('⌘');
    expect(keyGlyph('Alt', true)).toBe('⌥');
    expect(keyGlyph('Shift', true)).toBe('⇧');
  });

  it('maps modifier tokens to windows/linux words', () => {
    expect(keyGlyph('Mod', false)).toBe('Ctrl');
    expect(keyGlyph('Alt', false)).toBe('Alt');
    expect(keyGlyph('Shift', false)).toBe('Shift');
  });

  it('passes literal keys through unchanged on either platform', () => {
    expect(keyGlyph('N', true)).toBe('N');
    expect(keyGlyph('Esc', false)).toBe('Esc');
  });
});
