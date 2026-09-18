import { Link, useLocation, useNavigate } from '@tanstack/react-router';
import { FolderTree, Layers, Pin, PinOff, Search, Settings } from 'lucide-react';
import type React from 'react';
import { useEffect, useState } from 'react';
import { isTauri, useWebModeStatus } from '../../lib/web-mode.ts';
import { useUiStore } from '../../stores/ui.ts';
import { useWindowStore } from '../../stores/window.ts';
import { SettingsDialog } from '../settings/SettingsDialog.tsx';
import { Button } from '../ui/button.tsx';
import { Tooltip, TooltipContent, TooltipTrigger } from '../ui/tooltip.tsx';

/**
 * Left icon rail — the persistent navigation spine (IA-NAV-2/6). Gives the app a
 * "sense of place": Files / Search / Templates / Settings up top, Theme pinned
 * to the bottom. Every destination here is ALSO reachable from the command
 * palette (⌘K).
 */
export function ActivityRail(): React.JSX.Element {
  const sidebarView = useUiStore((s) => s.sidebarView);
  const setSidebarView = useUiStore((s) => s.setSidebarView);
  const sidebarCollapsed = useUiStore((s) => s.sidebarCollapsed);
  const toggleSidebar = useUiStore((s) => s.toggleSidebar);
  const setSidebarCollapsed = useUiStore((s) => s.setSidebarCollapsed);
  const alwaysOnTop = useWindowStore((s) => s.alwaysOnTop);
  const setAlwaysOnTop = useWindowStore((s) => s.setAlwaysOnTop);
  const location = useLocation();
  const navigate = useNavigate();
  const [settingsOpen, setSettingsOpen] = useState(false);
  const settingsOpenNonce = useUiStore((s) => s.settingsOpenNonce);
  const webModeStatus = useWebModeStatus();
  const webRunning = isTauri() && (webModeStatus.data?.running ?? false);
  useEffect(() => {
    if (settingsOpenNonce > 0) setSettingsOpen(true);
  }, [settingsOpenNonce]);
  const onWorkspace = location.pathname === '/';
  const onTemplates = location.pathname.startsWith('/templates');

  // "Files" toggles the file-tree sidebar when the Files view is already showing
  // on the workspace; otherwise it switches to the Files view and reveals it.
  // From another route it navigates back to the workspace first.
  const onFiles = () => {
    if (onWorkspace && sidebarView === 'files') {
      toggleSidebar();
    } else {
      setSidebarView('files');
      setSidebarCollapsed(false);
      if (!onWorkspace) navigate({ to: '/' });
    }
  };
  // "Search" switches the sidebar to the Search view and reveals it.
  const onSearch = () => {
    setSidebarView('search');
    setSidebarCollapsed(false);
    if (!onWorkspace) navigate({ to: '/' });
  };
  // Each rail button reads as "active" when its view is the one visible on the
  // workspace.
  const filesActive = onWorkspace && !sidebarCollapsed && sidebarView === 'files';
  const searchActive = onWorkspace && !sidebarCollapsed && sidebarView === 'search';

  return (
    <nav
      aria-label="Primary"
      className="flex w-12 shrink-0 flex-col items-center gap-1 border-r border-border bg-card py-2"
    >
      <RailButton label="Files" active={filesActive} onClick={onFiles}>
        <FolderTree className="h-5 w-5" />
      </RailButton>

      <RailButton label="Search" active={searchActive} onClick={onSearch}>
        <Search className="h-5 w-5" />
      </RailButton>

      <RailButton label="Templates" active={onTemplates} asChild>
        <Link to="/templates" aria-current={onTemplates ? 'page' : undefined}>
          <Layers className="h-5 w-5" />
        </Link>
      </RailButton>

      <div className="relative">
        <RailButton label="Settings" onClick={() => setSettingsOpen(true)}>
          <Settings className="h-5 w-5" />
        </RailButton>
        {webRunning && (
          <Tooltip>
            <TooltipTrigger asChild>
              <span
                role="img"
                className="absolute right-1 top-1 h-2 w-2 cursor-default rounded-full bg-mneme-cyan ring-1 ring-background"
                aria-label="Web access is active"
              />
            </TooltipTrigger>
            <TooltipContent side="right">Web access is on</TooltipContent>
          </Tooltip>
        )}
      </div>

      <div className="mt-auto flex flex-col items-center gap-1">
        <RailButton
          label={alwaysOnTop ? 'Floating on top (⌥⌘P)' : 'Float on top (⌥⌘P)'}
          active={alwaysOnTop}
          onClick={() => setAlwaysOnTop(!alwaysOnTop)}
        >
          {alwaysOnTop ? <Pin className="h-5 w-5" /> : <PinOff className="h-5 w-5" />}
        </RailButton>
      </div>

      <SettingsDialog open={settingsOpen} onOpenChange={setSettingsOpen} />
    </nav>
  );
}

function RailButton({
  label,
  active,
  onClick,
  asChild,
  children,
}: {
  label: string;
  active?: boolean;
  onClick?: () => void;
  asChild?: boolean;
  children: React.ReactNode;
}): React.JSX.Element {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button
          variant={active ? 'accent' : 'ghost'}
          size="icon"
          onClick={onClick}
          asChild={asChild}
          aria-label={label}
        >
          {children}
        </Button>
      </TooltipTrigger>
      <TooltipContent side="right">{label}</TooltipContent>
    </Tooltip>
  );
}
