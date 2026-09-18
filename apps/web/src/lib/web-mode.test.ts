// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  WEB_AUTH_EXPIRED_EVENT,
  bootstrapToken,
  clearWebToken,
  getWebToken,
  isTauri,
  notifyWebAuthExpired,
} from './web-mode.ts';

afterEach(() => {
  localStorage.clear();
  sessionStorage.clear();
  window.location.hash = '';
});

describe('web-mode transport', () => {
  it('detects non-Tauri context', () => {
    expect(isTauri()).toBe(false); // jsdom has no __TAURI_INTERNALS__
  });

  it('bootstraps the token from the URL fragment into localStorage then strips it', () => {
    window.location.hash = '#token=abc123';
    bootstrapToken();
    // Persisted in localStorage (survives new tabs / reopen within the profile).
    expect(localStorage.getItem('mneme.webToken')).toBe('abc123');
    expect(getWebToken()).toBe('abc123');
    expect(window.location.hash).toBe(''); // stripped (W7)
  });

  it('getWebToken reads from localStorage', () => {
    localStorage.setItem('mneme.webToken', 'cafe01');
    expect(getWebToken()).toBe('cafe01');
  });

  it('clearWebToken removes the persisted token', () => {
    localStorage.setItem('mneme.webToken', 'cafe01');
    clearWebToken();
    expect(getWebToken()).toBeNull();
    expect(localStorage.getItem('mneme.webToken')).toBeNull();
  });

  it('notifyWebAuthExpired dispatches the web-auth-expired event', () => {
    const handler = vi.fn();
    window.addEventListener(WEB_AUTH_EXPIRED_EVENT, handler);
    notifyWebAuthExpired();
    expect(handler).toHaveBeenCalledTimes(1);
    window.removeEventListener(WEB_AUTH_EXPIRED_EVENT, handler);
  });
});
