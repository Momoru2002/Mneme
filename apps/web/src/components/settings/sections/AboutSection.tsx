import { getVersion } from '@tauri-apps/api/app';
import type React from 'react';
import { useEffect, useState } from 'react';
import { toast } from 'sonner';
import { api } from '../../../lib/api-client.ts';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from '../../ui/alert-dialog.tsx';
import { Button } from '../../ui/button.tsx';
import { Field } from '../controls.tsx';

export function AboutSection(): React.JSX.Element {
  const [version, setVersion] = useState<string | null>(null);
  useEffect(() => {
    getVersion()
      .then(setVersion)
      .catch(() => setVersion(null));
  }, []);

  const onReveal = async () => {
    try {
      await api.system.revealLogs();
    } catch (e) {
      toast.error(e instanceof Error ? e.message : 'Failed to reveal logs');
    }
  };
  const onExport = async () => {
    try {
      const path = await api.system.exportDiagnostics();
      toast.success(`Diagnostic bundle written to ${path}`);
    } catch (e) {
      toast.error(e instanceof Error ? e.message : 'Export failed');
    }
  };
  const onWipe = async () => {
    try {
      await api.system.uninstallAndWipe();
      toast.success('Mneme data wiped. Quit and relaunch the app.');
    } catch (e) {
      toast.error(e instanceof Error ? e.message : 'Wipe failed');
    }
  };

  return (
    <div className="flex flex-col gap-4">
      <Field label="Version">
        <p className="text-sm text-muted-foreground">
          Mneme {version ?? '—'} <span className="text-xs">(local build)</span>
        </p>
      </Field>
      <Field label="Diagnostics">
        <div className="flex flex-wrap gap-2">
          <Button size="sm" variant="outline" onClick={onReveal}>
            Reveal logs
          </Button>
          <Button size="sm" variant="outline" onClick={onExport}>
            Export diagnostic bundle
          </Button>
        </div>
      </Field>
      <Field label="Danger zone">
        <AlertDialog>
          <AlertDialogTrigger asChild>
            <Button size="sm" variant="destructive" className="w-fit">
              Uninstall & wipe Mneme data
            </Button>
          </AlertDialogTrigger>
          <AlertDialogContent>
            <AlertDialogHeader>
              <AlertDialogTitle>Wipe all Mneme data?</AlertDialogTitle>
              <AlertDialogDescription>
                This permanently removes <span className="font-mono">~/.mneme</span> (settings, the
                wells registry, version-history metadata, diagnostics). Your note files in their
                well folders are NOT touched. Quit and relaunch Mneme afterward.
              </AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel>Cancel</AlertDialogCancel>
              <AlertDialogAction
                onClick={onWipe}
                className="bg-destructive text-destructive-foreground hover:brightness-110"
              >
                Wipe everything
              </AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
      </Field>
    </div>
  );
}
