import { Columns, Eye, FileText } from 'lucide-react';
import type React from 'react';
import type { ViewMode } from '../../stores/editor.ts';
import { Button } from '../ui/button.tsx';
import { Tooltip, TooltipContent, TooltipTrigger } from '../ui/tooltip.tsx';

type Mode = { value: ViewMode; label: string; Icon: typeof FileText };

const MODES: Mode[] = [
  { value: 'editor', label: 'Editor only', Icon: FileText },
  { value: 'split', label: 'Split', Icon: Columns },
  { value: 'preview', label: 'Preview only', Icon: Eye },
];

type Props = { value: ViewMode; onChange: (mode: ViewMode) => void };

export function ViewModeToggle({ value, onChange }: Props): React.JSX.Element {
  return (
    // biome-ignore lint/a11y/useSemanticElements: a toolbar of toggle buttons needs role="group"; <fieldset> would impose form-control semantics that don't apply.
    <div
      role="group"
      aria-label="View mode"
      className="inline-flex items-center rounded-md border border-border bg-muted p-0.5"
    >
      {MODES.map(({ value: v, label, Icon }) => (
        <Tooltip key={v}>
          <TooltipTrigger asChild>
            <Button
              variant={value === v ? 'default' : 'ghost'}
              size="icon"
              onClick={() => onChange(v)}
              className="h-7 w-7"
              aria-label={label}
              aria-pressed={value === v}
            >
              <Icon className="h-3.5 w-3.5" aria-hidden="true" />
            </Button>
          </TooltipTrigger>
          <TooltipContent>{label}</TooltipContent>
        </Tooltip>
      ))}
    </div>
  );
}
