import { describe, expect, it } from 'vitest';
import { resolveThemeName } from './theme.ts';

describe('resolveThemeName', () => {
  it('resolves auto by system preference', () => {
    expect(resolveThemeName('auto', false)).toBe('wellspring-dark');
    expect(resolveThemeName('auto', true)).toBe('wellspring-light');
  });
  it('aliases legacy dark/light', () => {
    expect(resolveThemeName('dark', false)).toBe('wellspring-dark');
    expect(resolveThemeName('light', false)).toBe('wellspring-light');
  });
  it('passes through a known theme name', () => {
    expect(resolveThemeName('midnight', false)).toBe('midnight');
    expect(resolveThemeName('sepia', true)).toBe('sepia');
  });
  it('falls back to wellspring-dark for an unknown name', () => {
    expect(resolveThemeName('bogus', false)).toBe('wellspring-dark');
  });
});
