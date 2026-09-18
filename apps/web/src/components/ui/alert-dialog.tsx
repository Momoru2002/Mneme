import * as AlertDialogPrimitive from '@radix-ui/react-alert-dialog';
import type React from 'react';
import { cn } from '../../lib/cn.ts';
import { buttonVariants } from './button.tsx';

export const AlertDialog = AlertDialogPrimitive.Root;
export const AlertDialogTrigger = AlertDialogPrimitive.Trigger;
export const AlertDialogPortal = AlertDialogPrimitive.Portal;

const AlertDialogOverlay = ({
  className,
  ...props
}: React.ComponentProps<typeof AlertDialogPrimitive.Overlay>): React.JSX.Element => (
  <AlertDialogPrimitive.Overlay
    className={cn(
      'fixed inset-0 z-50 bg-black/60 backdrop-blur-sm data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0',
      className,
    )}
    {...props}
  />
);

export const AlertDialogContent = ({
  className,
  ...props
}: React.ComponentProps<typeof AlertDialogPrimitive.Content>): React.JSX.Element => (
  <AlertDialogPortal>
    <AlertDialogOverlay />
    <AlertDialogPrimitive.Content
      className={cn(
        'fixed left-1/2 top-1/2 z-50 grid w-full max-w-lg -translate-x-1/2 -translate-y-1/2 gap-4 border border-border bg-card p-6 shadow-lg rounded-lg',
        className,
      )}
      {...props}
    />
  </AlertDialogPortal>
);

export const AlertDialogHeader = ({
  className,
  ...props
}: React.HTMLAttributes<HTMLDivElement>): React.JSX.Element => (
  <div className={cn('flex flex-col gap-2 text-center sm:text-left', className)} {...props} />
);

export const AlertDialogFooter = ({
  className,
  ...props
}: React.HTMLAttributes<HTMLDivElement>): React.JSX.Element => (
  <div
    className={cn('flex flex-col-reverse gap-2 sm:flex-row sm:justify-end', className)}
    {...props}
  />
);

export const AlertDialogTitle = ({
  className,
  ...props
}: React.ComponentProps<typeof AlertDialogPrimitive.Title>): React.JSX.Element => (
  <AlertDialogPrimitive.Title className={cn('text-lg font-semibold', className)} {...props} />
);

export const AlertDialogDescription = ({
  className,
  ...props
}: React.ComponentProps<typeof AlertDialogPrimitive.Description>): React.JSX.Element => (
  <AlertDialogPrimitive.Description
    className={cn('text-sm text-muted-foreground', className)}
    {...props}
  />
);

export const AlertDialogAction = ({
  className,
  ...props
}: React.ComponentProps<typeof AlertDialogPrimitive.Action>): React.JSX.Element => (
  <AlertDialogPrimitive.Action className={cn(buttonVariants(), className)} {...props} />
);

export const AlertDialogCancel = ({
  className,
  ...props
}: React.ComponentProps<typeof AlertDialogPrimitive.Cancel>): React.JSX.Element => (
  <AlertDialogPrimitive.Cancel
    className={cn(buttonVariants({ variant: 'outline' }), 'mt-2 sm:mt-0', className)}
    {...props}
  />
);
