import type React from 'react';
import { useEffect } from 'react';
import { usePrefs } from '../../lib/settings.ts';
import { useTree } from '../../lib/tree.ts';
import { ancestorsOf, useFileTreeStore } from '../../stores/file-tree.ts';
import { useWorkspaceStore } from '../../stores/workspace.ts';
import type { MoveTarget } from './MoveDialog.tsx';
import { TreeNode } from './TreeNode.tsx';

type Props = {
  wellId: string;
  /** M14: when true, write affordances (create/rename/delete/move) are hidden. */
  readOnly?: boolean;
  onCreateInFolder?: (folderPath: string, kind: 'file' | 'folder') => void;
  onMove?: (target: MoveTarget) => void;
};

export function FileTree({
  wellId,
  readOnly = false,
  onCreateInFolder,
  onMove,
}: Props): React.JSX.Element {
  const tree = useTree(wellId, '');
  const activePath = useWorkspaceStore((s) => s.selectActivePath());
  const expandAncestors = useFileTreeStore((s) => s.expandAncestors);
  const setHoverPath = useFileTreeStore((s) => s.setHoverPath);
  const { treeFontSize } = usePrefs();
  useEffect(() => {
    if (activePath?.wellId === wellId && activePath.path) {
      expandAncestors(wellId, ancestorsOf(activePath.path));
    }
  }, [activePath?.wellId, activePath?.path, wellId, expandAncestors]);

  if (tree.isLoading) {
    return <div className="px-3 py-2 text-xs text-muted-foreground">Loading tree…</div>;
  }
  if (tree.error) {
    return (
      <div className="px-3 py-2 text-xs text-destructive">{(tree.error as Error).message}</div>
    );
  }
  if (!tree.data || tree.data.entries.length === 0) {
    return (
      <div className="px-3 py-6 text-xs italic text-muted-foreground text-center">
        This well has no markdown files yet. Right-click anywhere or use the + button to create one.
      </div>
    );
  }
  return (
    <div
      className="flex flex-col py-1"
      style={{ fontSize: `${treeFontSize}px` }}
      onMouseLeave={() => setHoverPath(null)}
    >
      {tree.data.entries.map((entry) => (
        <TreeNode
          key={entry.path}
          wellId={wellId}
          name={entry.name}
          path={entry.path}
          type={entry.type}
          hasChildren={entry.hasChildren}
          readOnly={readOnly}
          onCreateInFolder={readOnly ? undefined : onCreateInFolder}
          onMove={readOnly ? undefined : onMove}
        />
      ))}
    </div>
  );
}
