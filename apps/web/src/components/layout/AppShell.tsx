import { useLocation, useNavigate } from '@tanstack/react-router';
import type React from 'react';
import { useEffect, useRef, useState } from 'react';
import { toast } from 'sonner';
import { useWells } from '../../lib/wells.ts';
import { useUiStore } from '../../stores/ui.ts';
import { useWindowStore } from '../../stores/window.ts';
import { useWorkspaceStore } from '../../stores/workspace.ts';
import { CommandPalette } from '../command/CommandPalette.tsx';
import { CreateInFolderDialog } from '../file-tree/CreateInFolderDialog.tsx';
import { TooltipProvider } from '../ui/tooltip.tsx';
import { ActivityRail } from './ActivityRail.tsx';
import { Header } from './Header.tsx';
import { Sidebar } from './Sidebar.tsx';

export function AppShell({ children }: { children: React.ReactNode }): React.JSX.Element {
  // The file-tree sidebar belongs to the workspace (the editor). Full-page
  // routes like Templates render in <main> without it.
  const onWorkspace = useLocation().pathname === '/';
  const navigate = useNavigate();
  const alwaysOnTop = useWindowStore((s) => s.alwaysOnTop);
  const [palette, setPalette] = useState<{ open: boolean; mode: 'commands' | 'files' }>({
    open: false,
    mode: 'commands',
  });

  const paletteOpenRef = useRef(false);
  useEffect(() => {
    paletteOpenRef.current = palette.open;
  }, [palette.open]);

  // New File (⌘/Ctrl+Alt+N + palette) opens a globally-mounted create dialog so
  // it works regardless of route or sidebar state. We read the active well via a
  // ref because the keydown listener is armed once (empty deps) and would
  // otherwise close over a stale value.
  const activeWellId = useWells().data?.activeWellId ?? null;
  const activeWellIdRef = useRef(activeWellId);
  useEffect(() => {
    activeWellIdRef.current = activeWellId;
  }, [activeWellId]);

  const [newFileOpen, setNewFileOpen] = useState(false);
  const newFileNonce = useUiStore((s) => s.newFileNonce);
  useEffect(() => {
    if (newFileNonce > 0) setNewFileOpen(true);
  }, [newFileNonce]);

  // Global key layer (IA-NAV-1, NATIVE-2): ⌘K opens commands, ⌘P opens
  // quick-open. Intercepted everywhere — including from inside the editor.
  // biome-ignore lint/correctness/useExhaustiveDependencies: the listener is armed ONCE for the app's lifetime; re-arming on every render is wasteful and unnecessary. `navigate` (from @tanstack/react-router) is stable across renders, so reading it in the closure is safe with the empty dep array.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey)) return;
      const key = e.key.toLowerCase();
      // ⌥⌘P — toggle Float (always-on-top). Works even with the palette open.
      if (e.altKey && key === 'p') {
        e.preventDefault();
        const s = useWindowStore.getState();
        s.setAlwaysOnTop(!s.alwaysOnTop);
        return;
      }
      // ⌘⇧F — open the Search sidebar view (focused).
      if (e.shiftKey && key === 'f') {
        e.preventDefault();
        navigate({ to: '/' });
        const ui = useUiStore.getState();
        ui.setSidebarView('search');
        ui.setSidebarCollapsed(false);
        return;
      }
      // While the palette is open, let cmdk own its keys (incl. its Ctrl+K/P
      // vim bindings); don't re-trigger or fight its navigation.
      if (paletteOpenRef.current) return;
      // ⌘/Ctrl+Alt+N — New File. Web-safe (avoids browser-reserved ⌘/Ctrl+N).
      if (e.altKey && key === 'n') {
        e.preventDefault();
        if (!activeWellIdRef.current) {
          toast.error('Select a well first');
          return;
        }
        useUiStore.getState().requestNewFile();
        return;
      }
      // ⌥⌘W / Ctrl+Alt+W — Close All Tabs. Web-safe (avoids browser-reserved
      // ⌘W / ⌘⇧W); mirrors the New File modifier pattern. Palette is the fallback.
      if (e.altKey && key === 'w') {
        e.preventDefault();
        useWorkspaceStore.getState().closeAllTabs();
        return;
      }
      if (key === 'k') {
        e.preventDefault();
        setPalette({ open: true, mode: 'commands' });
      } else if (key === 'p') {
        e.preventDefault();
        setPalette({ open: true, mode: 'files' });
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

  // Apply the Float (always-on-top) state to the desktop window: on mount the
  // persisted value is restored, and on every change the toggle/shortcut is
  // reflected. The Tauri window API only exists in the desktop webview; a catch
  // makes it a no-op elsewhere (and if the capability were missing).
  useEffect(() => {
    let cancelled = false;
    void import('@tauri-apps/api/window')
      .then(({ getCurrentWindow }) => {
        if (!cancelled) return getCurrentWindow().setAlwaysOnTop(alwaysOnTop);
      })
      .catch(() => {
        /* not in Tauri, or permission missing: no-op */
      });
    return () => {
      cancelled = true;
    };
  }, [alwaysOnTop]);

  return (
    <TooltipProvider delayDuration={300}>
      <div className="flex h-screen w-screen flex-col bg-background text-foreground">
        <a
          href="#main-content"
          className="sr-only focus:not-sr-only focus:absolute focus:left-3 focus:top-3 focus:z-50 focus:rounded-md focus:border focus:border-mneme-cyan focus:bg-card focus:px-3 focus:py-2 focus:text-sm focus:text-foreground focus:shadow-mneme-glow"
        >
          Skip to content
        </a>
        <Header />
        <div className="flex flex-1 overflow-hidden">
          <ActivityRail />
          {onWorkspace && <Sidebar />}
          <main id="main-content" tabIndex={-1} className="flex-1 overflow-auto focus:outline-none">
            {children}
          </main>
        </div>
        <CommandPalette
          open={palette.open}
          mode={palette.mode}
          onOpenChange={(open) => setPalette((p) => ({ ...p, open }))}
        />
        <CreateInFolderDialog
          wellId={activeWellId}
          open={newFileOpen}
          onOpenChange={setNewFileOpen}
          folderPath=""
          kind="file"
        />
      </div>
    </TooltipProvider>
  );
}
