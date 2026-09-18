import * as ContextMenuPrimitive from '@radix-ui/react-context-menu';
import type React from 'react';
import { cn } from '../../lib/cn.ts';

export const ContextMenu = ContextMenuPrimitive.Root;
export const ContextMenuTrigger = ContextMenuPrimitive.Trigger;
export const ContextMenuPortal = ContextMenuPrimitive.Portal;

export const ContextMenuContent = ({
  className,
  ...props
}: React.ComponentProps<typeof ContextMenuPrimitive.Content>): React.JSX.Element => (
  <ContextMenuPrimitive.Portal>
    <ContextMenuPrimitive.Content
      className={cn(
        'z-50 min-w-[12rem] overflow-hidden rounded-md border border-border bg-popover p-1 text-popover-foreground shadow-md',
        className,
      )}
      {...props}
    />
  </ContextMenuPrimitive.Portal>
);

export const ContextMenuItem = ({
  className,
  ...props
}: React.ComponentProps<typeof ContextMenuPrimitive.Item>): React.JSX.Element => (
  <ContextMenuPrimitive.Item
    className={cn(
      'relative flex cursor-default select-none items-center gap-2 rounded-sm px-2 py-1.5 text-sm outline-none transition-colors focus:bg-muted data-[disabled]:opacity-50 data-[disabled]:pointer-events-none',
      className,
    )}
    {...props}
  />
);

export const ContextMenuSeparator = ({
  className,
  ...props
}: React.ComponentProps<typeof ContextMenuPrimitive.Separator>): React.JSX.Element => (
  <ContextMenuPrimitive.Separator
    className={cn('-mx-1 my-1 h-px bg-border', className)}
    {...props}
  />
);
