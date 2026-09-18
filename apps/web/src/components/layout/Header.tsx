import { Link } from '@tanstack/react-router';
import type React from 'react';
import { Logo } from '../brand/Logo.tsx';
import { Separator } from '../ui/separator.tsx';
import { WellSelector } from './WellSelector.tsx';

export function Header(): React.JSX.Element {
  return (
    <header className="flex h-14 shrink-0 items-center gap-2 border-b border-border bg-card px-3 sm:gap-3 sm:px-4">
      <Link to="/" className="flex shrink-0 items-center gap-2" title="Mneme">
        <Logo size={28} variant="mark" />
        <span className="hidden font-extrabold tracking-widest text-mneme-cyan sm:inline">
          MNEME
        </span>
      </Link>
      <Separator orientation="vertical" className="hidden h-6 sm:block" />
      <div className="min-w-0 flex-1 sm:flex-initial">
        <WellSelector />
      </div>
    </header>
  );
}
