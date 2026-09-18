import { HighlightStyle, syntaxHighlighting } from '@codemirror/language';
import { EditorView } from '@codemirror/view';
import { tags } from '@lezer/highlight';

export const wellspringEditorTheme = EditorView.theme(
  {
    '&': {
      color: 'var(--color-mneme-text)',
      backgroundColor: 'var(--color-mneme-bg)',
      height: '100%',
      fontSize: 'var(--editor-font-size, 14px)',
    },
    '.cm-scroller': {
      fontFamily:
        'JetBrains Mono Variable, ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace',
      lineHeight: '1.6',
    },
    '.cm-content': {
      caretColor: 'var(--color-mneme-cyan)',
      padding: '16px 12px',
    },
    '.cm-cursor, .cm-dropCursor': {
      borderLeftColor: 'var(--color-mneme-cyan)',
      borderLeftWidth: '2px',
    },
    '&.cm-focused .cm-selectionBackground, ::selection': {
      backgroundColor: 'rgba(79, 209, 197, 0.18)',
    },
    '.cm-gutters': {
      backgroundColor: 'var(--color-mneme-bg)',
      color: 'var(--color-mneme-text-dim)',
      border: 'none',
      borderRight: '1px solid var(--color-mneme-border)',
    },
    '.cm-activeLine': {
      backgroundColor: 'rgba(40, 52, 73, 0.4)',
    },
    '.cm-activeLineGutter': {
      backgroundColor: 'rgba(40, 52, 73, 0.4)',
      color: 'var(--color-mneme-text-muted)',
    },
  },
  { dark: true },
);

const c = {
  cyan: '#4FD1C5',
  gold: '#D4A24E',
  text: '#E2E8F0',
  muted: '#94A3B8',
  dim: '#64748B',
  info: '#38BDF8',
  ok: '#22C55E',
  warn: '#F59E0B',
};

export const wellspringHighlight = HighlightStyle.define([
  { tag: tags.heading1, color: c.cyan, fontWeight: 'bold' },
  { tag: tags.heading2, color: c.cyan, fontWeight: 'bold' },
  { tag: tags.heading3, color: c.cyan, fontWeight: 'bold' },
  { tag: tags.heading4, color: c.cyan },
  { tag: tags.heading5, color: c.cyan },
  { tag: tags.heading6, color: c.cyan },
  { tag: tags.strong, color: c.gold, fontWeight: 'bold' },
  { tag: tags.emphasis, color: c.gold, fontStyle: 'italic' },
  { tag: tags.strikethrough, textDecoration: 'line-through', color: c.dim },
  { tag: tags.link, color: c.info, textDecoration: 'underline' },
  { tag: tags.url, color: c.info },
  { tag: tags.list, color: c.muted },
  { tag: tags.quote, color: c.muted, fontStyle: 'italic' },
  { tag: tags.monospace, color: c.gold, fontFamily: 'inherit' },
  { tag: tags.contentSeparator, color: c.dim },
  { tag: tags.comment, color: c.dim, fontStyle: 'italic' },
  { tag: tags.meta, color: c.muted },
  { tag: tags.keyword, color: c.cyan },
  { tag: tags.string, color: c.ok },
  { tag: tags.number, color: c.warn },
]);

export const wellspringHighlightExtension = syntaxHighlighting(wellspringHighlight);
