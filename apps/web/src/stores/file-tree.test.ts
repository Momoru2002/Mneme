import { beforeEach, describe, expect, it } from 'vitest';
import { ancestorsOf, insideOrEq, partializeFileTree, useFileTreeStore } from './file-tree.ts';

beforeEach(() => {
  useFileTreeStore.setState({ expandedByWell: {}, hoverPath: null });
});

describe('ancestorsOf', () => {
  it('returns ancestor folder paths of a node path', () => {
    expect(ancestorsOf('a/b/c.md')).toEqual(['a', 'a/b']);
    expect(ancestorsOf('x.md')).toEqual([]);
    expect(ancestorsOf('a/b')).toEqual(['a']);
  });
});

describe('insideOrEq', () => {
  it('is true for the folder itself and segment-descendants only', () => {
    expect(insideOrEq('a', 'a/b.md')).toBe(true);
    expect(insideOrEq('a/b', 'a/b')).toBe(true);
    expect(insideOrEq('a', 'ab/c.md')).toBe(false); // prefix but not a path segment
    expect(insideOrEq('a', null)).toBe(false);
  });
});

describe('file-tree store', () => {
  it('setExpanded adds and removes a path for a well', () => {
    useFileTreeStore.getState().setExpanded('w1', 'a', true);
    expect(useFileTreeStore.getState().expandedByWell.w1).toEqual(['a']);
    useFileTreeStore.getState().setExpanded('w1', 'a', false);
    expect(useFileTreeStore.getState().expandedByWell.w1).toEqual([]);
  });

  it('toggleExpanded flips presence', () => {
    useFileTreeStore.getState().toggleExpanded('w1', 'a');
    expect(useFileTreeStore.getState().expandedByWell.w1).toEqual(['a']);
    useFileTreeStore.getState().toggleExpanded('w1', 'a');
    expect(useFileTreeStore.getState().expandedByWell.w1).toEqual([]);
  });

  it('expandAncestors adds deduped without collapsing existing', () => {
    useFileTreeStore.getState().setExpanded('w1', 'a', true);
    useFileTreeStore.getState().expandAncestors('w1', ['a', 'a/b']);
    expect(useFileTreeStore.getState().expandedByWell.w1).toEqual(['a', 'a/b']);
  });

  it('collapseAll empties only the target well', () => {
    useFileTreeStore.getState().setExpanded('w1', 'a', true);
    useFileTreeStore.getState().setExpanded('w2', 'x', true);
    useFileTreeStore.getState().collapseAll('w1');
    expect(useFileTreeStore.getState().expandedByWell.w1).toEqual([]);
    expect(useFileTreeStore.getState().expandedByWell.w2).toEqual(['x']);
  });

  it('collapseAll is a no-op (same state ref) for an already-empty / never-seen well', () => {
    const before = useFileTreeStore.getState().expandedByWell;
    useFileTreeStore.getState().collapseAll('never-seen');
    // No phantom entry written, and the object identity is unchanged (no needless update).
    expect(useFileTreeStore.getState().expandedByWell).toBe(before);
    expect(useFileTreeStore.getState().expandedByWell['never-seen']).toBeUndefined();
  });

  it('partialize persists only expandedByWell (not hoverPath)', () => {
    useFileTreeStore.setState({ expandedByWell: { w1: ['a'] }, hoverPath: 'a/b.md' });
    expect(partializeFileTree(useFileTreeStore.getState())).toEqual({
      expandedByWell: { w1: ['a'] },
    });
  });
});
