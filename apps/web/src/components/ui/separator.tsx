import * as SeparatorPrimitive from '@radix-ui/react-separator';
import type React from 'react';
import { cn } from '../../lib/cn.ts';

export const Separator = ({
  className,
  orientation = 'horizontal',
  decorative = true,
  ...props
}: React.ComponentProps<typeof SeparatorPrimitive.Root>): React.JSX.Element => (
  <SeparatorPrimitive.Root
    decorative={decorative}
    orientation={orientation}
    className={cn(
      'shrink-0 bg-border',
      orientation === 'horizontal' ? 'h-px w-full' : 'h-full w-px',
      className,
    )}
    {...props}
  />
);
