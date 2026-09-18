import { EditorView } from '@codemirror/view';
import { FileText, Layers, OctagonAlert, RefreshCw, Save } from 'lucide-react';
import type React from 'react';
import { useCallback, useEffect, useRef, useState } from 'react';
import { Panel, PanelGroup, PanelResizeHandle } from 'react-resizable-panels';
import { insertTemplateAtCursor } from '../../lib/editor-buffer.ts';
import { useFileContent } from '../../lib/files.ts';
import { actionsOf, docKeyOf, useDocumentsStore } from '../../stores/documents.ts';
import { type ViewMode, useEditorStore } from '../../stores/editor.ts';
import { InsertTemplateDialog } from '../editor/InsertTemplateDialog.tsx';
import { MarkdownEditor, type MarkdownEditorHandle } from '../editor/MarkdownEditor.tsx';
import { MarkdownPreview } from '../preview/MarkdownPreview.tsx';
import { Button } from '../ui/button.tsx';
import { EditorStatusBar } from './EditorStatusBar.tsx';
import { FrontmatterPanel } from './FrontmatterPanel.tsx';
import { MergeDialog } from './MergeDialog.tsx';

type Props = { wellId: string; path: string; viewMode: ViewMode };

/**
 * One editor PANE for `{ wellId, path }`. It owns no save/conflict logic: it reads
 * the reactive `DocView` from the registry and drives the engine through the
 * `DocActions` the matching `DocumentHost` registered. `doc`/`actions` can be
 * undefined on the first render before the host registers — every read is guarded
 * and the pane renders an empty (buffer `''`) editor until the host primes.
 */
export function FileEditor({ wellId, path, viewMode }: Props): React.JSX.Element {
  const key = docKeyOf(wellId, path);
  const doc = useDocumentsStore((s) => s.docs[key]);
  const actions = actionsOf(key);
  const buffer = doc?.buffer ?? '';

  // Frontmatter only — the buffer/save state comes from the registry. React Query
  // dedupes this request with the host's useFileContent call, so it's cheap.
  const file = useFileContent(wellId, path);

  const [insertOpen, setInsertOpen] = useState(false);
  const editorRef = useRef<MarkdownEditorHandle | null>(null);

  const revealTarget = useEditorStore((s) => s.revealTarget);
  const setRevealTarget = useEditorStore((s) => s.setRevealTarget);

  // Consume a reveal target addressed to THIS doc (search jump). Ported from
  // EditorWorkbench:212-227, gated additionally on revealTarget.docKey === key.
  // The `doc?.ready` guard waits until this file's content is primed: on a file
  // switch the pane re-renders with the new `key` while buffer is still '' and the
  // OLD view exists, so firing now would jump to line 1 of an empty doc and burn
  // the target. `buffer` stays in deps as the trigger that re-fires after prime.
  // biome-ignore lint/correctness/useExhaustiveDependencies: `buffer` is a deliberate trigger so the jump runs after the buffer primes (the CodeMirror doc then holds the target line).
  useEffect(() => {
    if (!revealTarget || revealTarget.docKey !== key) return;
    if (!doc?.ready) return; // wait until this file's content is primed
    const view = editorRef.current?.view;
    if (!view) return;
    const lineNo = Math.min(Math.max(1, revealTarget.line), view.state.doc.lines);
    const line = view.state.doc.line(lineNo);
    const col = Math.min(Math.max(1, revealTarget.column), line.length + 1);
    const pos = line.from + (col - 1);
    view.dispatch({
      selection: { anchor: pos },
      effects: EditorView.scrollIntoView(pos, { y: 'center' }),
    });
    view.focus();
    setRevealTarget(null);
  }, [revealTarget, key, buffer, setRevealTarget, doc?.ready]);

  const onInsert = (rendered: string) => {
    const view = editorRef.current?.view;
    if (!view) {
      // Fallback: append (editor not mounted yet) — host handles cleanup.
      actions?.insertTemplate(rendered);
      return;
    }
    const sel = view.state.selection.main;
    const { doc: nextDoc, cursor } = insertTemplateAtCursor(buffer, sel.from, sel.to, rendered);
    actions?.onChange(nextDoc);
    // Place the caret where {{cursor}} was, after React re-renders the value.
    requestAnimationFrame(() => {
      editorRef.current?.view?.dispatch({ selection: { anchor: cursor } });
      editorRef.current?.view?.focus();
    });
  };

  // Stable handlers: read `actionsOf(key)` at CALL time so the ref never closes
  // over a render-time `actions` snapshot. FileEditor re-renders on every keystroke
  // (it subscribes to the registry buffer); a fresh handler ref would invalidate
  // MarkdownEditor's `extensions` useMemo and rebuild the whole CodeMirror config
  // per keystroke. Keyed on `[key]`, so the ref only changes on a file switch.
  const onChange = useCallback((next: string) => actionsOf(key)?.onChange(next), [key]);
  const onSave = useCallback(() => actionsOf(key)?.onForceSave(), [key]);

  return (
    <div className="flex h-full flex-col overflow-hidden">
      <header className="flex min-w-0 items-center gap-3 border-b border-border bg-card px-4 py-2">
        <FileText className="h-4 w-4 shrink-0 text-muted-foreground" />
        {/* Filename only — folder lives in the files panel; full path is the tooltip. */}
        <h1 className="min-w-0 flex-1 truncate font-mono text-sm text-foreground" title={path}>
          {path.slice(path.lastIndexOf('/') + 1).replace(/\.md$/, '')}
        </h1>
        <span className="ml-auto flex shrink-0 items-center gap-2 text-muted-foreground">
          <Button
            variant="ghost"
            size="icon"
            onClick={onSave}
            className="h-7 w-7"
            aria-label="Save now (⌘S)"
            title="Save now (⌘S)"
          >
            <Save className="h-3.5 w-3.5" aria-hidden="true" />
          </Button>
          <Button
            variant="accent"
            size="icon"
            onClick={() => setInsertOpen(true)}
            className="h-7 w-7"
            aria-label="Insert template"
            title="Insert template"
          >
            <Layers className="h-3.5 w-3.5" aria-hidden="true" />
          </Button>
        </span>
      </header>

      {doc?.status.kind === 'conflict' && (
        <ConflictBanner
          onReload={() => actions?.reloadFromDisk()}
          onOverwrite={() => actions?.overwriteDisk()}
          onResolve={() => actions?.openMerge()}
        />
      )}

      {file.data && <FrontmatterPanel data={file.data.frontmatter} />}

      <div className="flex flex-1 overflow-hidden">
        <div className="min-w-0 flex-1 overflow-hidden">
          {viewMode === 'editor' && (
            <MarkdownEditor ref={editorRef} value={buffer} onChange={onChange} onSave={onSave} />
          )}
          {viewMode === 'preview' && <MarkdownPreview source={buffer} />}
          {viewMode === 'split' && (
            <PanelGroup direction="horizontal">
              <Panel defaultSize={50} minSize={20}>
                <MarkdownEditor
                  ref={editorRef}
                  value={buffer}
                  onChange={onChange}
                  onSave={onSave}
                />
              </Panel>
              <PanelResizeHandle className="w-1 bg-border hover:bg-mneme-cyan transition-colors" />
              <Panel defaultSize={50} minSize={20}>
                <MarkdownPreview source={buffer} />
              </Panel>
            </PanelGroup>
          )}
        </div>
      </div>

      <EditorStatusBar status={doc?.status ?? { kind: 'idle' }} />

      <InsertTemplateDialog open={insertOpen} onOpenChange={setInsertOpen} onInsert={onInsert} />

      {/* The MergeDialog is driven by the reactive `mergeOpen` flag in the registry
          (set by the host's openMerge, cleared by resolveMerge/closeMerge). */}
      <MergeDialog
        open={doc?.mergeOpen ?? false}
        onOpenChange={(open) => {
          if (!open) actions?.closeMerge();
        }}
        diskContent={actions?.getMergeDisk() ?? ''}
        yourContent={buffer}
        onResolve={(merged) => actions?.resolveMerge(merged)}
      />
    </div>
  );
}

function ConflictBanner({
  onReload,
  onOverwrite,
  onResolve,
}: {
  onReload: () => void;
  onOverwrite: () => void;
  onResolve: () => void;
}): React.JSX.Element {
  return (
    <div
      role="alert"
      className="flex items-center gap-3 border-b border-mneme-danger/40 bg-mneme-danger/10 px-4 py-2 text-xs text-foreground"
    >
      <OctagonAlert className="h-4 w-4 shrink-0 text-mneme-danger" aria-hidden="true" />
      <span className="flex-1">
        This file changed on disk (edited outside Mneme). Your unsaved edits are safe here.
      </span>
      <Button variant="outline" size="sm" className="h-7 gap-1.5" onClick={onReload}>
        <RefreshCw className="h-3.5 w-3.5" aria-hidden="true" /> Reload disk
      </Button>
      <Button variant="outline" size="sm" className="h-7" onClick={onResolve}>
        Resolve…
      </Button>
      <Button variant="outline" size="sm" className="h-7" onClick={onOverwrite}>
        Overwrite
      </Button>
    </div>
  );
}
