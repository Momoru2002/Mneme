import { describe, expect, it } from 'vitest';
import { accentSoft, goldSoft, normalizeHex } from './accent.ts';

describe('normalizeHex', () => {
  it('expands 3-digit shorthand to 6 digits', () => {
    expect(normalizeHex('#abc')).toBe('#aabbcc');
    expect(normalizeHex('#f00')).toBe('#ff0000');
  });
  it('lowercases 6-digit hex so preset equality + soft variants stay valid', () => {
    expect(normalizeHex('#4FD1C5')).toBe('#4fd1c5');
    expect(accentSoft(normalizeHex('#4FD1C5'))).toBe('#4fd1c522');
  });
  it('leaves a canonical lowercase 6-digit hex unchanged', () => {
    expect(normalizeHex('#d4a24e')).toBe('#d4a24e');
  });
});

describe('accentSoft', () => {
  it('appends 22 alpha to a 6-digit hex', () => {
    expect(accentSoft('#4fd1c5')).toBe('#4fd1c522');
    expect(accentSoft('#fb7185')).toBe('#fb718522');
  });
});

describe('goldSoft', () => {
  it('appends 1a alpha to a 6-digit hex', () => {
    expect(goldSoft('#d4a24e')).toBe('#d4a24e1a');
  });
});
