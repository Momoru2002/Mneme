import { isTauri } from './web-mode.ts';

// Categories safe to expose in a plain browser (web mode): note-taking surfaces
// only. No credential/network/desktop-control surfaces.
export const BASE_CATEGORIES = [
  { id: 'general', label: 'General' },
  { id: 'editor', label: 'Editor' },
  { id: 'appearance', label: 'Appearance' },
  { id: 'wells', label: 'Wells & Permissions' },
  { id: 'keyboard', label: 'Keyboard' },
  { id: 'about', label: 'About' },
] as const;

// The full desktop list. `web-access` toggles the server itself, so it is
// desktop-only and hidden when the same bundle runs in a browser (web mode).
export const DESKTOP_CATEGORIES = [
  { id: 'general', label: 'General' },
  { id: 'editor', label: 'Editor' },
  { id: 'appearance', label: 'Appearance' },
  { id: 'wells', label: 'Wells & Permissions' },
  { id: 'keyboard', label: 'Keyboard' },
  { id: 'about', label: 'About' },
  { id: 'web-access', label: 'Web Access' },
] as const;

export type CategoryId = (typeof DESKTOP_CATEGORIES)[number]['id'];

/** Returns the full category list for desktop, or the browser-safe subset in web mode. */
export function resolveCategories(): typeof BASE_CATEGORIES | typeof DESKTOP_CATEGORIES {
  return isTauri() ? DESKTOP_CATEGORIES : BASE_CATEGORIES;
}
