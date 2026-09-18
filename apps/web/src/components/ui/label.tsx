import * as LabelPrimitive from '@radix-ui/react-label';
import type React from 'react';
import { cn } from '../../lib/cn.ts';

export const Label = ({
  className,
  ...props
}: React.ComponentProps<typeof LabelPrimitive.Root>): React.JSX.Element => (
  <LabelPrimitive.Root
    className={cn(
      'text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70',
      className,
    )}
    {...props}
  />
);
