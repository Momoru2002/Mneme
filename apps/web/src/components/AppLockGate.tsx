import { Lock } from 'lucide-react';
import type React from 'react';
import { useState } from 'react';
import { useAuthStatus, useSetPassword, useUnlock } from '../lib/auth.ts';
import { isTauri } from '../lib/web-mode.ts';
import { Button } from './ui/button.tsx';
import { Input } from './ui/input.tsx';

/**
 * Wraps the whole app. Renders `children` once unlocked (or immediately, on
 * anything other than the Tauri desktop webview — the app-lock has no
 * meaning in a browser tab hitting Web Mode's HTTP API; a leaked token there
 * is refused by the backend itself with no unlock path over that transport).
 */
export function AppLockGate({ children }: { children: React.ReactNode }): React.JSX.Element {
  if (!isTauri()) return <>{children}</>;
  return <TauriAppLockGate>{children}</TauriAppLockGate>;
}

function TauriAppLockGate({ children }: { children: React.ReactNode }): React.JSX.Element {
  const status = useAuthStatus();

  // First paint, before we know is_set_up/unlocked yet: render nothing
  // rather than flash the unlock screen then immediately replace it.
  if (status.isLoading || !status.data) {
    return <div className="h-screen w-screen bg-background" />;
  }

  if (!status.data.isSetUp) {
    return <CreatePasswordScreen />;
  }

  if (!status.data.unlocked) {
    return <UnlockScreen />;
  }

  return <>{children}</>;
}

function LockScreenShell({
  title,
  description,
  children,
}: {
  title: string;
  description: string;
  children: React.ReactNode;
}): React.JSX.Element {
  return (
    <div className="flex h-screen w-screen items-center justify-center bg-background p-4">
      <div className="flex w-full max-w-sm flex-col items-center gap-4 rounded-lg border border-border bg-card p-6 text-center shadow-lg">
        <span className="flex h-12 w-12 items-center justify-center rounded-full bg-muted">
          <Lock className="h-6 w-6 text-muted-foreground" aria-hidden="true" />
        </span>
        <h1 className="text-lg font-semibold tracking-tight">{title}</h1>
        <p className="text-sm text-muted-foreground">{description}</p>
        {children}
      </div>
    </div>
  );
}

function CreatePasswordScreen(): React.JSX.Element {
  const setPassword = useSetPassword();
  const [password, setPasswordValue] = useState('');
  const [confirm, setConfirm] = useState('');
  const [validationError, setValidationError] = useState<string | null>(null);

  function onSubmit(e: React.FormEvent): void {
    e.preventDefault();
    setValidationError(null);
    if (password.length < 8) {
      setValidationError('Use at least 8 characters.');
      return;
    }
    if (password !== confirm) {
      setValidationError('Passwords don\u2019t match.');
      return;
    }
    setPassword.mutate(password);
  }

  return (
    <LockScreenShell
      title="Create a password"
      description="Mneme locks itself on every launch. Choose a password — there's no recovery if you forget it, since nothing is stored except its hash."
    >
      <form onSubmit={onSubmit} className="flex w-full flex-col gap-2">
        <Input
          type="password"
          placeholder="New password"
          value={password}
          onChange={(e) => setPasswordValue(e.target.value)}
          autoFocus
        />
        <Input
          type="password"
          placeholder="Confirm password"
          value={confirm}
          onChange={(e) => setConfirm(e.target.value)}
        />
        {(validationError || setPassword.error) && (
          <p className="text-sm text-destructive">
            {validationError ??
              (setPassword.error instanceof Error
                ? setPassword.error.message
                : 'Failed to set password')}
          </p>
        )}
        <Button type="submit" disabled={setPassword.isPending} className="mt-2">
          {setPassword.isPending ? 'Setting up\u2026' : 'Set password'}
        </Button>
      </form>
    </LockScreenShell>
  );
}

function UnlockScreen(): React.JSX.Element {
  const unlock = useUnlock();
  const [password, setPasswordValue] = useState('');
  const [wrongPassword, setWrongPassword] = useState(false);

  function onSubmit(e: React.FormEvent): void {
    e.preventDefault();
    setWrongPassword(false);
    unlock.mutate(password, {
      onError: () => setWrongPassword(true),
      onSettled: () => setPasswordValue(''),
    });
  }

  return (
    <LockScreenShell title="Mneme is locked" description="Enter your password to continue.">
      <form onSubmit={onSubmit} className="flex w-full flex-col gap-2">
        <Input
          type="password"
          placeholder="Password"
          value={password}
          onChange={(e) => setPasswordValue(e.target.value)}
          autoFocus
        />
        {wrongPassword && <p className="text-sm text-destructive">Incorrect password.</p>}
        <Button type="submit" disabled={unlock.isPending || !password} className="mt-2">
          {unlock.isPending ? 'Unlocking\u2026' : 'Unlock'}
        </Button>
      </form>
    </LockScreenShell>
  );
}
