import { beforeEach, describe, expect, it } from 'vitest';
import { useSearchStore } from './search.ts';

beforeEach(() => {
  useSearchStore.setState({ options: { caseSensitive: false, wholeWord: false, regex: false } });
});

describe('search store — match options', () => {
  it('defaults to all off', () => {
    expect(useSearchStore.getState().options).toEqual({
      caseSensitive: false,
      wholeWord: false,
      regex: false,
    });
  });
  it('setOption flips one flag, leaving others', () => {
    useSearchStore.getState().setOption('regex', true);
    expect(useSearchStore.getState().options).toEqual({
      caseSensitive: false,
      wholeWord: false,
      regex: true,
    });
    useSearchStore.getState().setOption('caseSensitive', true);
    expect(useSearchStore.getState().options.caseSensitive).toBe(true);
    expect(useSearchStore.getState().options.regex).toBe(true);
  });
});
