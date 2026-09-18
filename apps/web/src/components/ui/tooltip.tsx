import * as TooltipPrimitive from '@radix-ui/react-tooltip';
import type React from 'react';
import { cn } from '../../lib/cn.ts';

export const TooltipProvider = TooltipPrimitive.Provider;
export const Tooltip = TooltipPrimitive.Root;
export const TooltipTrigger = TooltipPrimitive.Trigger;

export const TooltipContent = ({
  className,
  sideOffset = 4,
  ...props
}: React.ComponentProps<typeof TooltipPrimitive.Content>): React.JSX.Element => (
  <TooltipPrimitive.Portal>
    <TooltipPrimitive.Content
      sideOffset={sideOffset}
      className={cn(
        'z-50 overflow-hidden rounded-md border border-border bg-popover px-2 py-1 text-xs text-popover-foreground shadow-md',
        className,
      )}
      {...props}
    />
  </TooltipPrimitive.Portal>
);
