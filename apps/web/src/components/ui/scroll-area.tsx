import * as ScrollAreaPrimitive from '@radix-ui/react-scroll-area';
import type React from 'react';
import { cn } from '../../lib/cn.ts';

export const ScrollArea = ({
  className,
  children,
  ...props
}: React.ComponentProps<typeof ScrollAreaPrimitive.Root>): React.JSX.Element => (
  <ScrollAreaPrimitive.Root className={cn('relative overflow-hidden', className)} {...props}>
    <ScrollAreaPrimitive.Viewport className="h-full w-full rounded-[inherit]">
      {children}
    </ScrollAreaPrimitive.Viewport>
    <ScrollBar />
    <ScrollAreaPrimitive.Corner />
  </ScrollAreaPrimitive.Root>
);

const ScrollBar = ({
  className,
  orientation = 'vertical',
  ...props
}: React.ComponentProps<typeof ScrollAreaPrimitive.ScrollAreaScrollbar>): React.JSX.Element => (
  <ScrollAreaPrimitive.ScrollAreaScrollbar
    orientation={orientation}
    className={cn(
      'flex touch-none select-none transition-colors',
      orientation === 'vertical' && 'h-full w-2.5 border-l border-l-transparent p-px',
      orientation === 'horizontal' && 'h-2.5 flex-col border-t border-t-transparent p-px',
      className,
    )}
    {...props}
  >
    <ScrollAreaPrimitive.ScrollAreaThumb className="relative flex-1 rounded-full bg-border" />
  </ScrollAreaPrimitive.ScrollAreaScrollbar>
);
