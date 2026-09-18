import { useEffect, useRef } from 'react';
import { applyAccent, applyGold } from '../../lib/accent.ts';
import { useSettings } from '../../lib/settings.ts';
import { applyBackendTheme } from '../../lib/theme.ts';

export function SettingsBootstrap(): null {
  const s = useSettings();
  const appliedRef = useRef(false);
  useEffect(() => {
    if (appliedRef.current) return;
    if (s.data?.prefs) {
      applyBackendTheme(s.data.prefs.theme);
      applyAccent(s.data.prefs.accentColor ?? null);
      applyGold(s.data.prefs.goldColor ?? null);
      appliedRef.current = true;
    }
  }, [s.data?.prefs]);
  return null;
}
