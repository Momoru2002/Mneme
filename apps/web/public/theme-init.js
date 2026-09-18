// Pre-paint theme bootstrap (externalized so CSP needs no script 'unsafe-inline').
// Mirrors the small theme name->{mode,bg} map from lib/theme.ts THEMES so light
// themes don't flash a dark bg before globals.css loads.
(function () {
  try {
    var THEMES = {
      'wellspring-dark': { mode: 'dark', bg: '#0b1929' },
      'wellspring-light': { mode: 'light', bg: '#f8fafc' },
      midnight: { mode: 'dark', bg: '#0a0a12' },
      nord: { mode: 'dark', bg: '#2e3440' },
      sepia: { mode: 'light', bg: '#f3ead6' },
      rose: { mode: 'dark', bg: '#191724' },
      dracula: { mode: 'dark', bg: '#282a36' },
      gruvbox: { mode: 'dark', bg: '#282828' },
      catppuccin: { mode: 'dark', bg: '#1e1e2e' },
      'solarized-light': { mode: 'light', bg: '#fdf6e3' },
    };
    var pref = localStorage.getItem('mneme.theme') || 'wellspring-dark';
    var light = window.matchMedia('(prefers-color-scheme: light)').matches;
    var name =
      pref === 'auto'
        ? light
          ? 'wellspring-light'
          : 'wellspring-dark'
        : pref === 'dark'
          ? 'wellspring-dark'
          : pref === 'light'
            ? 'wellspring-light'
            : THEMES[pref]
              ? pref
              : 'wellspring-dark';
    var def = THEMES[name];
    var root = document.documentElement;
    root.dataset.theme = name;
    root.style.backgroundColor = def.bg;
    root.style.colorScheme = def.mode;
  } catch (e) {}
})();
