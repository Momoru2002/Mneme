import { CaseSensitive, Regex, Search, WholeWord, X } from 'lucide-react';
import type React from 'react';
import { useEffect, useRef, useState } from 'react';
import { useWells } from '../../lib/wells.ts';
import { useSearchStore } from '../../stores/search.ts';
import { SearchResults } from '../file-tree/SearchResults.tsx';
import { Button } from '../ui/button.tsx';

const DEBOUNCE_MS = 300;

export function SearchPanel(): React.JSX.Element {
  const wells = useWells();
  const activeWellId = wells.data?.activeWellId ?? null;
  const options = useSearchStore((s) => s.options);
  const setOption = useSearchStore((s) => s.setOption);
  const query = useSearchStore((s) => s.query);
  const setQuery = useSearchStore((s) => s.setQuery);

  const [input, setInput] = useState(query);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const inputRef = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  useEffect(() => {
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => setQuery(input), DEBOUNCE_MS);
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
    };
  }, [input, setQuery]);

  const clear = () => {
    setInput('');
    setQuery('');
  };

  const toggles: { key: keyof typeof options; label: string; Icon: typeof Search }[] = [
    { key: 'caseSensitive', label: 'Match case', Icon: CaseSensitive },
    { key: 'wholeWord', label: 'Whole word', Icon: WholeWord },
    { key: 'regex', label: 'Use regular expression', Icon: Regex },
  ];

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center justify-between px-3 py-2">
        <span className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          Search
        </span>
      </div>
      <div className="px-3 pb-2">
        <div className="flex items-center gap-2 rounded-md border border-border bg-background px-2 py-1.5 text-sm text-foreground focus-within:border-mneme-cyan focus-within:shadow-mneme-glow">
          <Search className="h-3.5 w-3.5 text-muted-foreground" />
          <input
            ref={inputRef}
            type="text"
            placeholder={activeWellId ? 'Search…' : 'Choose a well first'}
            value={input}
            onChange={(e) => setInput(e.target.value)}
            disabled={!activeWellId}
            onKeyDown={(e) => {
              if (e.key === 'Escape') clear();
            }}
            className="min-w-0 flex-1 bg-transparent outline-none placeholder:text-muted-foreground disabled:cursor-not-allowed"
          />
          {input && (
            <button
              type="button"
              onClick={clear}
              className="inline-flex h-6 w-6 items-center justify-center rounded-sm text-muted-foreground hover:text-foreground"
              aria-label="Clear search"
            >
              <X className="h-3.5 w-3.5" />
            </button>
          )}
        </div>
        <div className="mt-1.5 flex items-center gap-1">
          {toggles.map(({ key, label, Icon }) => (
            <Button
              key={key}
              type="button"
              variant={options[key] ? 'default' : 'ghost'}
              size="icon"
              className="h-6 w-6"
              aria-pressed={options[key]}
              aria-label={label}
              title={label}
              onClick={() => setOption(key, !options[key])}
            >
              <Icon className="h-3.5 w-3.5" aria-hidden="true" />
            </Button>
          ))}
        </div>
      </div>
      <div className="flex-1 overflow-auto">
        {activeWellId ? (
          <SearchResults wellId={activeWellId} query={query} options={options} />
        ) : (
          <p className="px-3 py-6 text-center text-xs text-muted-foreground">
            Choose or add a well to search.
          </p>
        )}
      </div>
    </div>
  );
}
