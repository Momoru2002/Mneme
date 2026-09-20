import { Lock } from 'lucide-react';
import type React from 'react';
import { useState } from 'react';
import { useChangePassword, useLockNow } from '../../../lib/auth.ts';
import { Button } from '../../ui/button.tsx';
import { Input } from '../../ui/input.tsx';
import { Field } from '../controls.tsx';

export function SecuritySection(): React.JSX.Element {
  const lockNow = useLockNow();
  const changePassword = useChangePassword();
  const [current, setCurrent] = useState('');
  const [next, setNext] = useState('');
  const [confirm, setConfirm] = useState('');
  const [validationError, setValidationError] = useState<string | null>(null);

  function onChangePassword(e: React.FormEvent): void {
    e.preventDefault();
    setValidationError(null);
    if (next.length < 8) {
      setValidationError('Use at least 8 characters.');
      return;
    }
    if (next !== confirm) {
      setValidationError('New passwords don\u2019t match.');
      return;
    }
    changePassword.mutate(
      { currentPassword: current, newPassword: next },
      {
        onSuccess: () => {
          setCurrent('');
          setNext('');
          setConfirm('');
        },
      },
    );
  }

  return (
    <div className="flex flex-col gap-6">
      <Field label="App lock">
        <p className="text-sm text-muted-foreground">
          Mneme requires this password on every launch, and refuses Web Access requests while locked
          — even with a valid link or QR code.
        </p>
        <div>
          <Button
            size="sm"
            variant="outline"
            onClick={() => lockNow.mutate()}
            disabled={lockNow.isPending}
          >
            <Lock className="h-3.5 w-3.5" />
            Lock now
          </Button>
        </div>
      </Field>

      <Field label="Change password">
        <form onSubmit={onChangePassword} className="flex flex-col gap-2">
          <Input
            type="password"
            placeholder="Current password"
            value={current}
            onChange={(e) => setCurrent(e.target.value)}
          />
          <Input
            type="password"
            placeholder="New password"
            value={next}
            onChange={(e) => setNext(e.target.value)}
          />
          <Input
            type="password"
            placeholder="Confirm new password"
            value={confirm}
            onChange={(e) => setConfirm(e.target.value)}
          />
          {validationError && <p className="text-sm text-destructive">{validationError}</p>}
          <div>
            <Button type="submit" size="sm" disabled={changePassword.isPending}>
              {changePassword.isPending ? 'Changing\u2026' : 'Change password'}
            </Button>
          </div>
        </form>
      </Field>
    </div>
  );
}
