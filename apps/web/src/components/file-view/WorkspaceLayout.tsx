import { Fragment, useMemo } from 'react';
import type React from 'react';
import { Panel, PanelGroup, PanelResizeHandle } from 'react-resizable-panels';
import type { LayoutNode } from '../../stores/workspace.ts';
import { useWorkspaceStore } from '../../stores/workspace.ts';
import { EditorGroup } from './EditorGroup.tsx';

export function WorkspaceLayout({ node }: { node: LayoutNode }): React.JSX.Element {
  const resize = useWorkspaceStore((s) => s.resize);

  // Debounce the resize write so localStorage.setItem (from zustand persist) only
  // fires once per drag settle (~150 ms trailing), not on every animation frame.
  // useMemo is called unconditionally here — before the early return — to satisfy
  // the rules of hooks. Each split-node instance gets its own timer closure.
  const debouncedResize = useMemo(() => {
    let timer: ReturnType<typeof setTimeout> | null = null;
    return (sizes: number[]) => {
      if (timer) clearTimeout(timer);
      timer = setTimeout(() => resize(node.id, sizes), 150);
    };
  }, [node.id, resize]);

  if (node.type === 'group') return <EditorGroup group={node} />;

  return (
    <PanelGroup
      direction={node.direction === 'row' ? 'horizontal' : 'vertical'}
      onLayout={debouncedResize}
      // Stable id so react-resizable-panels keeps sizing across re-renders.
      id={node.id}
    >
      {node.children.map((child, i) => (
        <Fragment key={child.id}>
          {i > 0 && (
            <PanelResizeHandle
              className={
                node.direction === 'row'
                  ? 'w-1 bg-border hover:bg-mneme-cyan transition-colors'
                  : 'h-1 bg-border hover:bg-mneme-cyan transition-colors'
              }
            />
          )}
          <Panel defaultSize={node.sizes[i]} minSize={15} id={child.id} order={i}>
            <WorkspaceLayout node={child} />
          </Panel>
        </Fragment>
      ))}
    </PanelGroup>
  );
}
