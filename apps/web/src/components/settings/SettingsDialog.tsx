import type { UserPrefsPatch } from '@mneme/shared';
import type React from 'react';
import { useEffect, useMemo, useState } from 'react';
import { cn } from '../../lib/cn.ts';
import type { CategoryId } from '../../lib/settings-categories.ts';
import { resolveCategories } from '../../lib/settings-categories.ts';
import { useSettings, useUpdateSettings } from '../../lib/settings.ts';
import { useUiStore } from '../../stores/ui.ts';
import { Button } from '../ui/button.tsx';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../ui/dialog.tsx';
import type { PrefsGet, PrefsSet } from './controls.tsx';
import { AboutSection } from './sections/AboutSection.tsx';
import { AppearanceSection } from './sections/AppearanceSection.tsx';
import { EditorSection } from './sections/EditorSection.tsx';
import { GeneralSection } from './sections/GeneralSection.tsx';
import { KeyboardSection } from './sections/KeyboardSection.tsx';
import { SecuritySection } from './sections/SecuritySection.tsx';
import { WebAccessSection } from './sections/WebAccessSection.tsx';
import { WellsSection } from './sections/WellsSection.tsx';

type Props = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
};

export function SettingsDialog({ open, onOpenChange }: Props): React.JSX.Element {
  const settings = useSettings();
  const update = useUpdateSettings();
  const [draft, setDraft] = useState<UserPrefsPatch>({});
  const [active, setActive] = useState<CategoryId>('general');
  const settingsTarget = useUiStore((s) => s.settingsTarget);
  const clearSettingsTarget = useUiStore((s) => s.clearSettingsTarget);

  // Web Access category is only available inside the Tauri desktop app.
  const categories = useMemo(() => resolveCategories(), []);

  // Reset the prefs draft exactly once per open — NOT on a retarget, so an
  // in-flight Editor/Appearance edit isn't wiped when the dialog is re-pointed
  // at a category (e.g. "Manage Wells…").
  useEffect(() => {
    if (open) setDraft({});
  }, [open]);

  // Jump to a requested category when the dialog is (re)opened with a target.
  useEffect(() => {
    if (open && settingsTarget) {
      if (categories.some((c) => c.id === settingsTarget)) {
        setActive(settingsTarget as CategoryId);
      }
      clearSettingsTarget();
    }
  }, [open, settingsTarget, clearSettingsTarget, categories]);

  const prefs = settings.data?.prefs;
  const get: PrefsGet = (key) => (draft[key] !== undefined ? draft[key] : prefs?.[key]);
  const set: PrefsSet = (key, value) => setDraft((p) => ({ ...p, [key]: value }));

  const onSave = async () => {
    if (Object.keys(draft).length === 0) {
      onOpenChange(false);
      return;
    }
    await update.mutateAsync(draft);
    onOpenChange(false);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-3xl">
        <DialogHeader>
          <DialogTitle>Settings</DialogTitle>
          <DialogDescription>Configure Mneme.</DialogDescription>
        </DialogHeader>

        <div className="flex h-[60vh] gap-4">
          <nav className="flex w-40 shrink-0 flex-col gap-1 border-r border-border pr-2">
            {categories.map((c) => (
              <button
                key={c.id}
                type="button"
                onClick={() => setActive(c.id)}
                className={cn(
                  'rounded-md px-3 py-2 text-left text-sm hover:bg-muted',
                  active === c.id && 'bg-muted font-medium text-foreground',
                )}
              >
                {c.label}
              </button>
            ))}
          </nav>
          <div className="min-w-0 flex-1 overflow-y-auto pr-2">
            {active === 'general' && <GeneralSection />}
            {active === 'editor' && <EditorSection get={get} set={set} />}
            {active === 'appearance' && <AppearanceSection get={get} set={set} />}
            {active === 'wells' && <WellsSection />}
            {active === 'keyboard' && <KeyboardSection />}
            {active === 'about' && <AboutSection />}
            {active === 'web-access' && <WebAccessSection />}
            {active === 'security' && <SecuritySection />}
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={onSave} disabled={update.isPending}>
            {update.isPending ? 'Saving…' : 'Save'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
