import { useEffect, useState } from 'react';

export type ThemeMode = 'dark' | 'light';
export type ThemeDef = {
  name: string;
  label: string;
  mode: ThemeMode;
  bg: string;
  accent: string;
  gold: string;
};

/**
 * Curated themes. `bg`/`accent`/`gold` mirror each theme's tokens in
 * `wellspring.css` and seed the picker swatches + paint-time anti-flash.
 */
export const THEMES: ThemeDef[] = [
  {
    name: 'wellspring-dark',
    label: 'Wellspring Dark',
    mode: 'dark',
    bg: '#0b1929',
    accent: '#4fd1c5',
    gold: '#d4a24e',
  },
  {
    name: 'wellspring-light',
    label: 'Wellspring Light',
    mode: 'light',
    bg: '#f8fafc',
    accent: '#0d9488',
    gold: '#b45309',
  },
  {
    name: 'midnight',
    label: 'Midnight',
    mode: 'dark',
    bg: '#0a0a12',
    accent: '#818cf8',
    gold: '#a78bfa',
  },
  { name: 'nord', label: 'Nord', mode: 'dark', bg: '#2e3440', accent: '#88c0d0', gold: '#ebcb8b' },
  {
    name: 'sepia',
    label: 'Sepia',
    mode: 'light',
    bg: '#f3ead6',
    accent: '#2c7a6f',
    gold: '#a0682c',
  },
  { name: 'rose', label: 'Rosé', mode: 'dark', bg: '#191724', accent: '#ebbcba', gold: '#f6c177' },
  {
    name: 'dracula',
    label: 'Dracula',
    mode: 'dark',
    bg: '#282a36',
    accent: '#bd93f9',
    gold: '#ffb86c',
  },
  {
    name: 'gruvbox',
    label: 'Gruvbox',
    mode: 'dark',
    bg: '#282828',
    accent: '#8ec07c',
    gold: '#fabd2f',
  },
  {
    name: 'catppuccin',
    label: 'Catppuccin Mocha',
    mode: 'dark',
    bg: '#1e1e2e',
    accent: '#cba6f7',
    gold: '#f9e2af',
  },
  {
    name: 'solarized-light',
    label: 'Solarized Light',
    mode: 'light',
    bg: '#fdf6e3',
    accent: '#2aa198',
    gold: '#7f6400',
  },
];

const DEFAULT_DARK = 'wellspring-dark';
const DEFAULT_LIGHT = 'wellspring-light';
const STORAGE_KEY = 'mneme.theme';
const byName = new Map(THEMES.map((t) => [t.name, t]));

function systemPrefersLight(): boolean {
  if (typeof window === 'undefined') return false;
  return window.matchMedia('(prefers-color-scheme: light)').matches;
}

/** Resolve a stored pref ('auto' | legacy 'dark'/'light' | a theme name | unknown) to a concrete theme name. */
export function resolveThemeName(pref: string, prefersLight: boolean): string {
  if (pref === 'auto') return prefersLight ? DEFAULT_LIGHT : DEFAULT_DARK;
  if (pref === 'dark') return DEFAULT_DARK;
  if (pref === 'light') return DEFAULT_LIGHT;
  return byName.has(pref) ? pref : DEFAULT_DARK;
}

function readStored(): string {
  if (typeof window === 'undefined') return DEFAULT_DARK;
  return window.localStorage.getItem(STORAGE_KEY) ?? DEFAULT_DARK;
}

function apply(pref: string): void {
  if (typeof document === 'undefined') return;
  const name = resolveThemeName(pref, systemPrefersLight());
  const def = byName.get(name) ?? byName.get(DEFAULT_DARK);
  if (!def) return;
  const root = document.documentElement;
  root.dataset.theme = name;
  root.style.backgroundColor = def.bg;
  root.style.colorScheme = def.mode;
}

let _pref: string = readStored();
apply(_pref);

const listeners = new Set<(t: string) => void>();

export function setTheme(next: string): void {
  _pref = next;
  if (typeof window !== 'undefined') window.localStorage.setItem(STORAGE_KEY, next);
  apply(next);
  for (const l of listeners) l(next);
}

/** One-way pull of the backend theme into the local store on app boot. */
export function applyBackendTheme(theme: string): void {
  setTheme(theme);
}

export function useTheme(): {
  theme: string;
  effectiveMode: ThemeMode;
  effectiveAccent: string;
  effectiveGold: string;
  setTheme: (t: string) => void;
  themes: ThemeDef[];
} {
  const [theme, setLocal] = useState<string>(_pref);
  // Bumped when the OS theme flips in auto mode, so `effectiveMode` consumers
  // (Sonner/Mermaid) re-render even though `theme` stays 'auto'.
  const [, forceTick] = useState(0);

  useEffect(() => {
    const cb = (t: string) => setLocal(t);
    listeners.add(cb);
    return () => {
      listeners.delete(cb);
    };
  }, []);

  // Re-apply on system preference change while in auto mode.
  useEffect(() => {
    if (theme !== 'auto') return;
    const mq = window.matchMedia('(prefers-color-scheme: light)');
    const onChange = () => {
      apply('auto');
      forceTick((n) => n + 1);
    };
    mq.addEventListener('change', onChange);
    return () => mq.removeEventListener('change', onChange);
  }, [theme]);

  const resolved =
    byName.get(resolveThemeName(theme, systemPrefersLight())) ?? byName.get(DEFAULT_DARK);
  return {
    theme,
    effectiveMode: resolved ? resolved.mode : 'dark',
    effectiveAccent: resolved ? resolved.accent : '#4fd1c5',
    effectiveGold: resolved ? resolved.gold : '#d4a24e',
    setTheme,
    themes: THEMES,
  };
}
