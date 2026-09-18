import { beforeEach, describe, expect, it } from 'vitest';
import { useWindowStore } from './window.ts';

beforeEach(() => {
  useWindowStore.setState({ alwaysOnTop: false });
});

describe('window store — always-on-top (Float)', () => {
  it('defaults to not floating', () => {
    expect(useWindowStore.getState().alwaysOnTop).toBe(false);
  });

  it('setAlwaysOnTop flips the flag', () => {
    useWindowStore.getState().setAlwaysOnTop(true);
    expect(useWindowStore.getState().alwaysOnTop).toBe(true);
    useWindowStore.getState().setAlwaysOnTop(false);
    expect(useWindowStore.getState().alwaysOnTop).toBe(false);
  });
});
