import { MergeView } from '@codemirror/merge';
import { EditorState } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import type React from 'react';
import { useCallback, useEffect, useRef } from 'react';
import { Button } from '../ui/button.tsx';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../ui/dialog.tsx';

type Props = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Current on-disk content (left, read-only). */
  diskContent: string;
  /** The user's unsaved buffer (right, editable). */
  yourContent: string;
  /** Called with the reconciled right-pane text when the user saves the merge. */
  onResolve: (merged: string) => void;
};

/**
 * Two-pane chunk merge (STATES-1, user-chosen "full merge"): Disk on the left
 * (read-only), Your version on the right (editable). Per-chunk gutter arrows
 * (revertControls 'a-to-b') let the user pull the disk side over; the right pane
 * is also freely editable. Built on @codemirror/merge's MergeView.
 */
export function MergeDialog({
  open,
  onOpenChange,
  diskContent,
  yourContent,
  onResolve,
}: Props): React.JSX.Element {
  const hostRef = useRef<HTMLDivElement | null>(null);
  const mergeRef = useRef<MergeView | null>(null);

  // Build the MergeView once when the dialog opens. diskContent/yourContent are
  // snapshotted at open time on purpose: re-keying on them would destroy and
  // rebuild the view (discarding in-progress merge edits) if the parent buffer
  // changed while the dialog is open. The merged result is read live from
  // mergeRef on save, so the snapshot is only the initial doc.
  // biome-ignore lint/correctness/useExhaustiveDependencies: open is the lifecycle key; diskContent/yourContent are intentional open-time snapshots (see comment).
  useEffect(() => {
    if (!open || !hostRef.current) return;
    const view = new MergeView({
      parent: hostRef.current,
      revertControls: 'a-to-b',
      a: {
        doc: diskContent,
        extensions: [EditorView.editable.of(false), EditorState.readOnly.of(true)],
      },
      b: { doc: yourContent, extensions: [EditorView.lineWrapping] },
    });
    mergeRef.current = view;
    return () => {
      view.destroy();
      mergeRef.current = null;
    };
  }, [open]);

  const onSaveMerged = useCallback(() => {
    const merged = mergeRef.current?.b.state.doc.toString() ?? yourContent;
    onResolve(merged);
    onOpenChange(false);
  }, [yourContent, onResolve, onOpenChange]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-4xl">
        <DialogHeader>
          <DialogTitle>Resolve conflict</DialogTitle>
          <DialogDescription>
            Left is the version on disk (read-only). Right is your version — edit it freely or use
            the gutter arrows to pull changes across, then Save merged.
          </DialogDescription>
        </DialogHeader>
        <div ref={hostRef} className="max-h-[60vh] overflow-auto rounded-md border border-border" />
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={onSaveMerged}>Save merged</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
