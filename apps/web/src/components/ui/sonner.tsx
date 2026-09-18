import type React from 'react';
import { Toaster as Sonner } from 'sonner';
import { useTheme } from '../../lib/theme.ts';

export function Toaster(props: React.ComponentProps<typeof Sonner>): React.JSX.Element {
  const { effectiveMode } = useTheme();
  return (
    <Sonner
      theme={effectiveMode}
      className="toaster group"
      toastOptions={{
        classNames: {
          toast:
            'group toast group-[.toaster]:bg-card group-[.toaster]:text-card-foreground group-[.toaster]:border-border group-[.toaster]:shadow-lg',
          description: 'group-[.toast]:text-muted-foreground',
          actionButton: 'group-[.toast]:bg-primary group-[.toast]:text-primary-foreground',
          cancelButton: 'group-[.toast]:bg-muted group-[.toast]:text-muted-foreground',
        },
      }}
      {...props}
    />
  );
}
