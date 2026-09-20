// @vitest-environment node
import { describe, expect, it, vi } from 'vitest';

// ---------------------------------------------------------------------------
// Test: Web Access category filtering based on isTauri()
// The SettingsDialog builds its category list at render time from isTauri().
// We verify that resolveCategories() (the shared module both Dialog and this
// test consume) excludes 'web-access' in browser context and includes it in
// Tauri desktop context.
// ---------------------------------------------------------------------------

vi.mock('./web-mode.ts', () => ({
  isTauri: vi.fn(() => false),
  getWebToken: vi.fn(() => null),
  bootstrapToken: vi.fn(),
  WEB_MODE_KEY: ['web-mode', 'status'],
  useWebModeStatus: vi.fn(),
  useEnableWebMode: vi.fn(),
  useDisableWebMode: vi.fn(),
  useOpenInBrowser: vi.fn(),
}));

import { BASE_CATEGORIES, resolveCategories } from './settings-categories.ts';
import { isTauri } from './web-mode.ts';

describe('Settings category list — Web Access visibility', () => {
  it('excludes "Web Access" when isTauri() is false (browser context)', () => {
    vi.mocked(isTauri).mockReturnValue(false);
    const cats = resolveCategories();
    const ids = cats.map((c) => c.id);
    expect(ids).not.toContain('web-access');
    // Base categories are all present.
    expect(ids).toContain('general');
    expect(ids).toContain('about');
  });

  it('includes "Web Access" when isTauri() is true (desktop context)', () => {
    vi.mocked(isTauri).mockReturnValue(true);
    const cats = resolveCategories();
    const ids = cats.map((c) => c.id);
    expect(ids).toContain('web-access');
    // The web-access entry has the right label.
    const entry = cats.find((c) => c.id === 'web-access');
    expect(entry?.label).toBe('Web Access');
  });

  it('desktop is a superset of base; web-access is desktop-only', () => {
    vi.mocked(isTauri).mockReturnValue(true);
    const desktopIds = resolveCategories().map((c) => c.id);
    const baseIds = BASE_CATEGORIES.map((c) => c.id);

    // Every browser-safe (base) category is also present on desktop.
    for (const id of baseIds) expect(desktopIds).toContain(id);

    // The web-toggle surface and the app-lock are both desktop-only — present
    // on desktop, ABSENT from the browser list (so a browser user can't reach
    // either: web-access controls the server itself, and security has no
    // unlock path over that transport anyway).
    for (const id of ['web-access', 'security']) {
      expect(desktopIds).toContain(id);
      expect(baseIds).not.toContain(id);
    }

    // web-access and security stay last, in that order, in the desktop list.
    expect(desktopIds.slice(-2)).toEqual(['web-access', 'security']);
  });

  it('browser context hides web-access (desktop-only surface)', () => {
    vi.mocked(isTauri).mockReturnValue(false);
    const ids = resolveCategories().map((c) => c.id);
    expect(ids).not.toContain('web-access');
  });
});
