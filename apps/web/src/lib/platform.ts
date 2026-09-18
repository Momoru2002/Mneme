/**
 * Platform detection used ONLY for display (rendering shortcut glyphs). The
 * key-handling layer stays on `e.metaKey || e.ctrlKey` — never branch behaviour
 * on this. `keyGlyph` is a pure token→label mapper so it can be unit-tested
 * without a DOM; `isMacPlatform` reads the environment.
 */
export function isMacPlatform(): boolean {
  if (typeof navigator === 'undefined') return false;
  const uaPlatform = (navigator as Navigator & { userAgentData?: { platform?: string } })
    .userAgentData?.platform;
  const platform = uaPlatform ?? navigator.platform ?? '';
  return /mac/i.test(platform);
}

const MAC_GLYPHS: Record<string, string> = { Mod: '⌘', Alt: '⌥', Shift: '⇧' };
const PC_LABELS: Record<string, string> = { Mod: 'Ctrl', Alt: 'Alt', Shift: 'Shift' };

/** Map a symbolic key token to its on-screen label for the given platform. */
export function keyGlyph(token: string, isMac: boolean): string {
  const table = isMac ? MAC_GLYPHS : PC_LABELS;
  return table[token] ?? token;
}
