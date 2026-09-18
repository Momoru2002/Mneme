import { markdown, markdownLanguage } from '@codemirror/lang-markdown';
import { indentUnit } from '@codemirror/language';
import { EditorState, Prec } from '@codemirror/state';
import { EditorView, keymap } from '@codemirror/view';
import CodeMirror, { type ReactCodeMirrorRef } from '@uiw/react-codemirror';
import type React from 'react';
import { forwardRef, useMemo } from 'react';
import { usePrefs } from '../../lib/settings.ts';
import { wellspringEditorTheme, wellspringHighlightExtension } from './wellspring-theme.ts';

/** Imperative handle the workbench uses to read the live EditorView. */
export type MarkdownEditorHandle = ReactCodeMirrorRef;

type Props = {
  value: string;
  onChange?: (next: string) => void;
  /** Fired on Cmd/Ctrl+S — forces an immediate save (EDITING-2). */
  onSave?: () => void;
  readOnly?: boolean;
};

// Extensions that never depend on prefs (lineWrapping/indent are added per-prefs below).
const STABLE_EXTENSIONS = [
  markdown({ base: markdownLanguage }),
  wellspringEditorTheme,
  wellspringHighlightExtension,
];

const STATIC_BASIC_SETUP = {
  highlightActiveLine: true,
  highlightActiveLineGutter: true,
  foldGutter: true,
  autocompletion: false,
  bracketMatching: true,
  closeBrackets: false,
  searchKeymap: true,
  indentOnInput: true,
} as const;

export const MarkdownEditor = forwardRef<MarkdownEditorHandle, Props>(function MarkdownEditor(
  { value, onChange, onSave, readOnly = false },
  ref,
): React.JSX.Element {
  const { fontSize, tabWidth, lineNumbers, wordWrap, indentStyle } = usePrefs();

  // Rebuilds (→ CodeMirror reconfigures in place, preserving doc/cursor) only
  // when one of these prefs or onSave changes — never on a plain keystroke.
  const extensions = useMemo(() => {
    const saveKeymap = Prec.highest(
      keymap.of([
        {
          key: 'Mod-s',
          run: () => {
            if (!onSave) return false;
            onSave();
            return true;
          },
        },
      ]),
    );
    const indent = indentStyle === 'tabs' ? '\t' : ' '.repeat(tabWidth);
    return [
      saveKeymap,
      ...STABLE_EXTENSIONS,
      indentUnit.of(indent),
      EditorState.tabSize.of(tabWidth),
      ...(wordWrap ? [EditorView.lineWrapping] : []),
    ];
  }, [onSave, tabWidth, indentStyle, wordWrap]);

  const basicSetup = useMemo(() => ({ ...STATIC_BASIC_SETUP, lineNumbers }), [lineNumbers]);

  return (
    <CodeMirror
      ref={ref}
      value={value}
      onChange={onChange}
      readOnly={readOnly}
      basicSetup={basicSetup}
      extensions={extensions}
      style={
        {
          height: '100%',
          overflow: 'hidden',
          '--editor-font-size': `${fontSize}px`,
        } as React.CSSProperties
      }
      theme="none"
    />
  );
});
