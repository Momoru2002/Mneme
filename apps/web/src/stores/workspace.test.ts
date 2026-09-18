import { beforeEach, describe, expect, it } from 'vitest';
import { shallow } from 'zustand/shallow';
import {
  type GroupNode,
  type SplitNode,
  initialWorkspace,
  makeGroup,
  mergeWorkspace,
  useWorkspaceStore,
} from './workspace.ts';

beforeEach(() => {
  useWorkspaceStore.setState(initialWorkspace());
});

const tab = (wellId: string, path: string) => ({ wellId, path });

describe('workspace — tabs in a single group', () => {
  it('starts with one empty root group that is active', () => {
    const s = useWorkspaceStore.getState();
    expect(s.root.type).toBe('group');
    if (s.root.type === 'group') {
      expect(s.root.tabs).toEqual([]);
      expect(s.root.activeTab).toBeNull();
      expect(s.activeGroupId).toBe(s.root.id);
    }
  });

  it('openInActiveGroup adds a tab and makes it active', () => {
    useWorkspaceStore.getState().openInActiveGroup(tab('w1', 'a.md'));
    const root = useWorkspaceStore.getState().root;
    if (root.type !== 'group') throw new Error('expected group');
    expect(root.tabs).toEqual([tab('w1', 'a.md')]);
    expect(root.activeTab).toBe(0);
  });

  it('opening an already-open tab just activates it (no duplicate)', () => {
    const s = () => useWorkspaceStore.getState();
    s().openInActiveGroup(tab('w1', 'a.md'));
    s().openInActiveGroup(tab('w1', 'b.md'));
    s().openInActiveGroup(tab('w1', 'a.md'));
    const root = s().root;
    if (root.type !== 'group') throw new Error('expected group');
    expect(root.tabs).toHaveLength(2);
    expect(root.activeTab).toBe(0);
  });

  it('closeTab removes it and re-points activeTab', () => {
    const s = () => useWorkspaceStore.getState();
    s().openInActiveGroup(tab('w1', 'a.md'));
    s().openInActiveGroup(tab('w1', 'b.md'));
    const gid = s().activeGroupId;
    s().closeTab(gid, 0); // close a.md
    const root = s().root;
    if (root.type !== 'group') throw new Error('expected group');
    expect(root.tabs).toEqual([tab('w1', 'b.md')]);
    expect(root.activeTab).toBe(0);
  });

  it('closing the last tab leaves an empty active root group', () => {
    const s = () => useWorkspaceStore.getState();
    s().openInActiveGroup(tab('w1', 'a.md'));
    const gid = s().activeGroupId;
    s().closeTab(gid, 0);
    const root = s().root;
    if (root.type !== 'group') throw new Error('expected group');
    expect(root.tabs).toEqual([]);
    expect(root.activeTab).toBeNull();
  });

  it('setActiveTab changes the active index', () => {
    const s = () => useWorkspaceStore.getState();
    s().openInActiveGroup(tab('w1', 'a.md'));
    s().openInActiveGroup(tab('w1', 'b.md'));
    s().setActiveTab(s().activeGroupId, 0);
    const root = s().root;
    if (root.type !== 'group') throw new Error('expected group');
    expect(root.activeTab).toBe(0);
  });

  it('selectOpenPaths returns every open path for a well', () => {
    const s = () => useWorkspaceStore.getState();
    s().openInActiveGroup(tab('w1', 'a.md'));
    s().openInActiveGroup(tab('w1', 'b.md'));
    s().openInActiveGroup(tab('w2', 'c.md'));
    expect(s().selectOpenPaths('w1')).toEqual(new Set(['a.md', 'b.md']));
  });

  it('selectOpenPaths is shallow-stable on unchanged state (guards React #185 at TreeNode)', () => {
    // selectOpenPaths allocates a fresh Set per call, so a *bare* zustand
    // subscription hands useSyncExternalStore a new reference every render and
    // spins into an infinite re-render (React #185). TreeNode therefore wraps the
    // selector in `useShallow`; this locks the contract that makes that work:
    // for unchanged state the Sets differ by reference but are shallow-equal.
    const s = () => useWorkspaceStore.getState();
    s().openInActiveGroup(tab('w1', 'a.md'));
    s().openInActiveGroup(tab('w1', 'b.md'));
    const first = s().selectOpenPaths('w1');
    const second = s().selectOpenPaths('w1');
    expect(first).not.toBe(second); // fresh allocation each call (the footgun)
    expect(shallow(first, second)).toBe(true); // ...but shallow-equal → useShallow stays stable
  });

  it('selectActivePath returns the active group active tab, or null', () => {
    const s = () => useWorkspaceStore.getState();
    expect(s().selectActivePath()).toBeNull();
    s().openInActiveGroup(tab('w1', 'a.md'));
    s().openInActiveGroup(tab('w1', 'b.md'));
    expect(s().selectActivePath()).toEqual(tab('w1', 'b.md'));
    s().setActiveTab(s().activeGroupId, 0);
    expect(s().selectActivePath()).toEqual(tab('w1', 'a.md'));
  });

  it('removePathFromTabs drops matching tabs across the tree', () => {
    const s = () => useWorkspaceStore.getState();
    s().openInActiveGroup(tab('w1', 'a.md'));
    s().openInActiveGroup(tab('w1', 'b.md'));
    s().removePathFromTabs('w1', 'a.md');
    expect(s().selectOpenPaths('w1')).toEqual(new Set(['b.md']));
  });

  it('renamePathInTabs rewrites matching tabs', () => {
    const s = () => useWorkspaceStore.getState();
    s().openInActiveGroup(tab('w1', 'a.md'));
    s().renamePathInTabs('w1', 'a.md', 'renamed.md');
    expect(s().selectOpenPaths('w1')).toEqual(new Set(['renamed.md']));
  });

  it('removePathFromTabs keeps the active tab pointing at the same file when an earlier tab is removed', () => {
    const s = () => useWorkspaceStore.getState();
    s().openInActiveGroup(tab('w1', 'a.md'));
    s().openInActiveGroup(tab('w1', 'b.md'));
    s().openInActiveGroup(tab('w1', 'c.md'));
    s().setActiveTab(s().activeGroupId, 1); // active = b.md
    s().removePathFromTabs('w1', 'a.md'); // remove earlier tab
    const root = s().root;
    if (root.type !== 'group') throw new Error('expected group');
    expect(root.tabs).toEqual([tab('w1', 'b.md'), tab('w1', 'c.md')]);
    expect(root.activeTab).toBe(0); // still b.md, shifted down
  });

  it('closeTab closing the active tab activates the next one', () => {
    const s = () => useWorkspaceStore.getState();
    s().openInActiveGroup(tab('w1', 'a.md'));
    s().openInActiveGroup(tab('w1', 'b.md'));
    s().openInActiveGroup(tab('w1', 'c.md'));
    s().setActiveTab(s().activeGroupId, 1);
    s().closeTab(s().activeGroupId, 1);
    const root = s().root;
    if (root.type !== 'group') throw new Error('expected group');
    expect(root.tabs).toEqual([tab('w1', 'a.md'), tab('w1', 'c.md')]);
    expect(root.activeTab).toBe(1);
  });

  it('closeTab closing a tab after active does not move activeTab', () => {
    const s = () => useWorkspaceStore.getState();
    s().openInActiveGroup(tab('w1', 'a.md'));
    s().openInActiveGroup(tab('w1', 'b.md'));
    s().setActiveTab(s().activeGroupId, 0);
    s().closeTab(s().activeGroupId, 1);
    const root = s().root;
    if (root.type !== 'group') throw new Error('expected group');
    expect(root.tabs).toEqual([tab('w1', 'a.md')]);
    expect(root.activeTab).toBe(0);
  });
});

describe('workspace — legacy migration', () => {
  it('migrateLegacyOpenPath seeds the root group once when empty', () => {
    const s = () => useWorkspaceStore.getState();
    s().migrateLegacyOpenPath('w1', 'note.md');
    expect(s().selectOpenPaths('w1')).toEqual(new Set(['note.md']));
    // idempotent: a second call with a populated workspace is a no-op
    s().migrateLegacyOpenPath('w1', 'other.md');
    expect(s().selectOpenPaths('w1')).toEqual(new Set(['note.md']));
  });
});

describe('workspace — defensive hydration', () => {
  it('mergeWorkspace falls back to a fresh workspace when persisted root is malformed', () => {
    const current = useWorkspaceStore.getState();
    const merged = mergeWorkspace({ root: { type: 'bogus' }, activeGroupId: 5 }, current);
    expect(merged.root.type).toBe('group');
    expect(typeof merged.activeGroupId).toBe('string');
  });

  it('mergeWorkspace falls back to fresh on undefined/null persisted', () => {
    const current = useWorkspaceStore.getState();
    expect(mergeWorkspace(undefined, current).root.type).toBe('group');
    expect(mergeWorkspace(null, current).root.type).toBe('group');
  });

  it('mergeWorkspace restores a valid tree and reseeds idSeq above the persisted max', () => {
    const persistedRoot: SplitNode = {
      type: 'split',
      id: 's2',
      direction: 'row',
      sizes: [50, 50],
      children: [
        { type: 'group', id: 'g7', tabs: [], activeTab: null, viewMode: 'editor' } as GroupNode,
        { type: 'group', id: 'g3', tabs: [], activeTab: null, viewMode: 'editor' } as GroupNode,
      ],
    };
    const merged = mergeWorkspace(
      { root: persistedRoot, activeGroupId: 'g7' },
      useWorkspaceStore.getState(),
    );
    expect(merged.root).toBe(persistedRoot);
    expect(merged.activeGroupId).toBe('g7');
    // A subsequently generated group id must have a numeric suffix > 7 so it
    // can never collide with the restored `g7`.
    const suffix = Number(/\d+$/.exec(makeGroup().id)?.[0]);
    expect(suffix).toBeGreaterThan(7);
  });
});

describe('workspace — splitting', () => {
  it('splitGroup wraps the group in a split with a new empty active sibling', () => {
    const s = () => useWorkspaceStore.getState();
    s().openInActiveGroup(tab('w1', 'a.md'));
    const original = s().activeGroupId;
    s().splitGroup(original, 'row');
    const root = s().root;
    expect(root.type).toBe('split');
    if (root.type !== 'split') throw new Error('expected split');
    expect(root.direction).toBe('row');
    expect(root.children).toHaveLength(2);
    expect(root.sizes).toEqual([50, 50]);
    // first child is the original group, second is the new empty active one
    const [c0, c1] = root.children;
    if (!c0 || !c1) throw new Error('expected 2 children');
    if (c0.type !== 'group' || c1.type !== 'group') throw new Error('expected groups');
    expect(c0.id).toBe(original);
    expect(c1.tabs).toEqual([]);
    expect(s().activeGroupId).toBe(c1.id);
  });

  it('opening a file after split lands in the new active group', () => {
    const s = () => useWorkspaceStore.getState();
    s().openInActiveGroup(tab('w1', 'a.md'));
    s().splitGroup(s().activeGroupId, 'row');
    s().openInActiveGroup(tab('w1', 'b.md'));
    expect(s().selectOpenPaths('w1')).toEqual(new Set(['a.md', 'b.md']));
  });

  it('closing the last tab of a split group collapses the split (promotes sibling)', () => {
    const s = () => useWorkspaceStore.getState();
    s().openInActiveGroup(tab('w1', 'a.md'));
    s().splitGroup(s().activeGroupId, 'row');
    s().openInActiveGroup(tab('w1', 'b.md')); // in the new group
    const newGroup = s().activeGroupId;
    s().closeGroup(newGroup);
    const root = s().root;
    expect(root.type).toBe('group'); // collapsed back to a single group
    if (root.type !== 'group') throw new Error('expected group');
    expect(root.tabs).toEqual([tab('w1', 'a.md')]);
    expect(s().activeGroupId).toBe(root.id);
  });

  it('resize updates the sizes of a split node', () => {
    const s = () => useWorkspaceStore.getState();
    s().openInActiveGroup(tab('w1', 'a.md'));
    s().splitGroup(s().activeGroupId, 'row');
    const root = s().root;
    if (root.type !== 'split') throw new Error('expected split');
    s().resize(root.id, [30, 70]);
    const after = useWorkspaceStore.getState().root;
    if (after.type !== 'split') throw new Error('expected split');
    expect(after.sizes).toEqual([30, 70]);
  });

  it('splitting a nested group keeps the rest of the tree intact', () => {
    const s = () => useWorkspaceStore.getState();
    s().openInActiveGroup(tab('w1', 'a.md'));
    s().splitGroup(s().activeGroupId, 'row'); // root: split[g(a), g()]
    const second = s().activeGroupId;
    s().splitGroup(second, 'col'); // second becomes split[g(), g()]
    const root = s().root;
    if (root.type !== 'split') throw new Error('expected split');
    expect(root.direction).toBe('row');
    expect(root.children).toHaveLength(2);
    const right = root.children[1];
    if (!right) throw new Error('expected right child');
    expect(right.type).toBe('split');
    if (right.type !== 'split') throw new Error('expected nested split');
    expect(right.direction).toBe('col');
  });

  it('closing 1 of 3 panes renormalises sizes (does NOT collapse to single group)', () => {
    const s = () => useWorkspaceStore.getState();
    s().openInActiveGroup(tab('w1', 'a.md'));
    const g0 = s().activeGroupId;
    s().splitGroup(g0, 'row'); // split[g0, g1]
    const g1 = s().activeGroupId;
    s().splitGroup(g1, 'row'); // split[g0, split[g1, g2]]
    s().closeGroup(g1); // leaves split[g0, g2], renormalised
    const root = s().root;
    expect(root.type).toBe('split');
    if (root.type !== 'split') throw new Error('expected split');
    expect(root.children).toHaveLength(2);
    expect(root.sizes).toEqual([50, 50]);
    const activeStillInTree =
      root.children[0]?.id === s().activeGroupId || root.children[1]?.id === s().activeGroupId;
    expect(activeStillInTree).toBe(true);
  });
});

describe('workspace — close all tabs', () => {
  it('closeAllTabs resets a multi-tab single group to one empty active group', () => {
    const s = () => useWorkspaceStore.getState();
    s().openInActiveGroup(tab('w1', 'a.md'));
    s().openInActiveGroup(tab('w1', 'b.md'));
    s().closeAllTabs();
    const root = s().root;
    expect(root.type).toBe('group');
    if (root.type !== 'group') throw new Error('expected group');
    expect(root.tabs).toEqual([]);
    expect(root.activeTab).toBeNull();
    expect(s().activeGroupId).toBe(root.id);
  });

  it('closeAllTabs collapses a split layout to a single empty group', () => {
    const s = () => useWorkspaceStore.getState();
    s().openInActiveGroup(tab('w1', 'a.md'));
    s().splitGroup(s().activeGroupId, 'row');
    s().openInActiveGroup(tab('w1', 'b.md'));
    s().closeAllTabs();
    const root = s().root;
    expect(root.type).toBe('group');
    if (root.type !== 'group') throw new Error('expected group');
    expect(root.tabs).toEqual([]);
    expect(s().selectOpenPaths('w1')).toEqual(new Set());
    expect(s().activeGroupId).toBe(root.id);
  });
});
