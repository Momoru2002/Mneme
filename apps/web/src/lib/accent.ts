/**
 * Canonicalize a hex color to lowercase 6-digit form (`#rrggbb`). The in-app hex
 * field (react-colorful's HexColorInput) can emit shorthand (`#abc`) or uppercase
 * (`#4FD1C5`); normalizing at the source keeps the derived `*-soft` alpha variants
 * valid CSS and keeps equality checks against the lowercase preset list working.
 */
export function normalizeHex(hex: string): string {
  const h = hex.replace(/^#/, '').toLowerCase();
  const full = h.length === 3 ? h.replace(/./g, (c) => c + c) : h;
  return `#${full}`;
}

/** The translucent variant of an accent: a 6-digit hex + '22' alpha. */
export function accentSoft(hex: string): string {
  return `${hex}22`;
}

/** Apply (or clear) the user accent override on the document root. */
export function applyAccent(color: string | null): void {
  if (typeof document === 'undefined') return;
  const root = document.documentElement;
  if (color) {
    root.style.setProperty('--color-mneme-cyan', color);
    root.style.setProperty('--color-mneme-cyan-soft', accentSoft(color));
  } else {
    root.style.removeProperty('--color-mneme-cyan');
    root.style.removeProperty('--color-mneme-cyan-soft');
  }
}

/** The translucent variant of the gold/secondary accent: a 6-digit hex + '1a' alpha. */
export function goldSoft(hex: string): string {
  return `${hex}1a`;
}

/** Apply (or clear) the user secondary (gold) accent override on the document root. */
export function applyGold(color: string | null): void {
  if (typeof document === 'undefined') return;
  const root = document.documentElement;
  if (color) {
    root.style.setProperty('--color-mneme-gold', color);
    root.style.setProperty('--color-mneme-gold-soft', goldSoft(color));
  } else {
    root.style.removeProperty('--color-mneme-gold');
    root.style.removeProperty('--color-mneme-gold-soft');
  }
}
