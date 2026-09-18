import * as AvatarPrimitive from '@radix-ui/react-avatar';
import type React from 'react';
import { cn } from '../../lib/cn.ts';

export const Avatar = ({
  className,
  ...props
}: React.ComponentProps<typeof AvatarPrimitive.Root>): React.JSX.Element => (
  <AvatarPrimitive.Root
    className={cn('relative flex h-8 w-8 shrink-0 overflow-hidden rounded-full', className)}
    {...props}
  />
);

export const AvatarImage = ({
  className,
  ...props
}: React.ComponentProps<typeof AvatarPrimitive.Image>): React.JSX.Element => (
  <AvatarPrimitive.Image className={cn('aspect-square h-full w-full', className)} {...props} />
);

export const AvatarFallback = ({
  className,
  ...props
}: React.ComponentProps<typeof AvatarPrimitive.Fallback>): React.JSX.Element => (
  <AvatarPrimitive.Fallback
    className={cn(
      'flex h-full w-full items-center justify-center rounded-full bg-secondary text-sm font-semibold',
      className,
    )}
    {...props}
  />
);
