import type { UserPrefsPatch } from '@mneme/shared';
import type React from 'react';
import { Label } from '../ui/label.tsx';

/** Read a draft-or-saved pref value (draft wins). */
export type PrefsGet = <K extends keyof UserPrefsPatch>(key: K) => UserPrefsPatch[K] | undefined;
/** Stage a pref change into the draft. */
export type PrefsSet = <K extends keyof UserPrefsPatch>(key: K, value: UserPrefsPatch[K]) => void;

export function Field({
  label,
  children,
}: { label: string; children: React.ReactNode }): React.JSX.Element {
  return (
    <div className="flex flex-col gap-1">
      <Label>{label}</Label>
      {children}
    </div>
  );
}

export function Toggle({
  label,
  value,
  onChange,
}: {
  label: string;
  value: boolean;
  onChange: (v: boolean) => void;
}): React.JSX.Element {
  return (
    <label className="flex items-center justify-between gap-3 rounded-md border border-border bg-background px-3 py-2">
      <span className="text-sm">{label}</span>
      <input
        type="checkbox"
        checked={value}
        onChange={(e) => onChange(e.target.checked)}
        className="h-4 w-4 accent-mneme-cyan"
      />
    </label>
  );
}
