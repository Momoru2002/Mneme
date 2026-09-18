import { beforeEach, describe, expect, it } from 'vitest';
import { selectViewMode, useEditorStore } from './editor.ts';

beforeEach(() => {
  useEditorStore.setState({ defaultViewMode: 'editor' });
});

describe('editor store — global view mode', () => {
  it('selectViewMode returns the global view mode', () => {
    useEditorStore.setState({ defaultViewMode: 'split' });
    expect(selectViewMode(useEditorStore.getState())).toBe('split');
  });

  it('setDefaultViewMode works even when no file is open', () => {
    useEditorStore.getState().setDefaultViewMode('preview');
    expect(selectViewMode(useEditorStore.getState())).toBe('preview');
  });

  it('setDefaultViewMode (Settings) sets the same global mode', () => {
    useEditorStore.getState().setDefaultViewMode('split');
    expect(selectViewMode(useEditorStore.getState())).toBe('split');
  });

  it('setRevealTarget sets and clears the transient target', () => {
    useEditorStore.getState().setRevealTarget({ docKey: 'w1::a.md', line: 12, column: 3 });
    expect(useEditorStore.getState().revealTarget).toEqual({
      docKey: 'w1::a.md',
      line: 12,
      column: 3,
    });
    useEditorStore.getState().setRevealTarget(null);
    expect(useEditorStore.getState().revealTarget).toBeNull();
  });
});
