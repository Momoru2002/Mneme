import type React from 'react';
import { useCallback, useEffect, useRef, useState } from 'react';
import { useUiStore } from '../../stores/ui.ts';
import { FilesPanel } from './FilesPanel.tsx';
import { SearchPanel } from './SearchPanel.tsx';

const STORAGE_KEY = 'mneme.sidebar.width';
const MIN_WIDTH = 200;
const MAX_WIDTH = 640;
const DEFAULT_WIDTH = 256;

function readStoredWidth(): number {
  if (typeof window === 'undefined') return DEFAULT_WIDTH;
  const raw = window.localStorage.getItem(STORAGE_KEY);
  const parsed = raw ? Number.parseInt(raw, 10) : Number.NaN;
  if (Number.isFinite(parsed) && parsed >= MIN_WIDTH && parsed <= MAX_WIDTH) return parsed;
  return DEFAULT_WIDTH;
}

export function Sidebar(): React.JSX.Element {
  // Sidebar visibility is owned by the UI store so the rail's "Files" button and
  // this panel's own collapse button drive the same state (VS Code pattern). The
  // rail also picks which view (Files vs. Search) the shell renders.
  const collapsed = useUiStore((s) => s.sidebarCollapsed);
  const sidebarView = useUiStore((s) => s.sidebarView);
  const [width, setWidth] = useState<number>(readStoredWidth);
  const dragStartXRef = useRef<number | null>(null);
  const dragStartWidthRef = useRef<number>(DEFAULT_WIDTH);

  useEffect(() => {
    window.localStorage.setItem(STORAGE_KEY, String(width));
  }, [width]);

  const onResizeStart = useCallback(
    (e: React.MouseEvent<HTMLDivElement>) => {
      e.preventDefault();
      dragStartXRef.current = e.clientX;
      dragStartWidthRef.current = width;
      const onMove = (ev: MouseEvent) => {
        if (dragStartXRef.current == null) return;
        const dx = ev.clientX - dragStartXRef.current;
        const next = Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, dragStartWidthRef.current + dx));
        setWidth(next);
      };
      const onUp = () => {
        dragStartXRef.current = null;
        document.removeEventListener('mousemove', onMove);
        document.removeEventListener('mouseup', onUp);
        document.body.style.cursor = '';
        document.body.style.userSelect = '';
      };
      document.addEventListener('mousemove', onMove);
      document.addEventListener('mouseup', onUp);
      document.body.style.cursor = 'col-resize';
      document.body.style.userSelect = 'none';
    },
    [width],
  );

  // Keyboard operation for the resize separator (WCAG 2.1.1 + 2.5.7): a focused
  // user resizes with arrows / Home / End instead of a pointer drag.
  const onResizeKey = useCallback((e: React.KeyboardEvent<HTMLDivElement>) => {
    const STEP = 16;
    let next: number | null = null;
    if (e.key === 'ArrowLeft') next = -STEP;
    else if (e.key === 'ArrowRight') next = STEP;
    if (next != null) {
      e.preventDefault();
      setWidth((w) => Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, w + (next as number))));
      return;
    }
    if (e.key === 'Home') {
      e.preventDefault();
      setWidth(MIN_WIDTH);
    } else if (e.key === 'End') {
      e.preventDefault();
      setWidth(MAX_WIDTH);
    }
  }, []);

  // Collapsed: hide the whole panel. The activity rail (always visible to the
  // left) provides the "Files" button to bring it back — no redundant second
  // icon strip. Open files are preserved (the workspace store holds the tabs),
  // so reopening restores the highlight.
  if (collapsed) return <></>;

  return (
    <aside
      className="relative flex shrink-0 flex-col border-r border-border bg-card"
      style={{ width: `${width}px` }}
    >
      {sidebarView === 'search' ? <SearchPanel /> : <FilesPanel />}

      {/* Resize handle on the right edge — pointer drag + keyboard (arrows/Home/End). */}
      <div
        role="separator"
        aria-orientation="vertical"
        aria-label="Resize sidebar"
        aria-valuemin={MIN_WIDTH}
        aria-valuemax={MAX_WIDTH}
        aria-valuenow={width}
        tabIndex={0}
        onMouseDown={onResizeStart}
        onKeyDown={onResizeKey}
        className="absolute right-0 top-0 z-10 h-full w-1 cursor-col-resize hover:bg-mneme-cyan/40 focus-visible:bg-mneme-cyan/60 focus-visible:outline-none active:bg-mneme-cyan/60"
      />
    </aside>
  );
}
