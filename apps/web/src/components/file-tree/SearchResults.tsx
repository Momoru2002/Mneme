import type { SearchOptions, SearchResult } from '@mneme/shared';
import { FileText, Search } from 'lucide-react';
import type React from 'react';
import { useSearch } from '../../lib/search.ts';
import { docKeyOf } from '../../stores/documents.ts';
import { useEditorStore } from '../../stores/editor.ts';
import { useWorkspaceStore } from '../../stores/workspace.ts';

type Props = {
  wellId: string;
  query: string;
  options: SearchOptions;
};

function highlight(text: string, needle: string): React.JSX.Element[] {
  if (!needle) return [<span key="0">{text}</span>];
  const re = new RegExp(`(${needle.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')})`, 'gi');
  return text.split(re).map((part, i) => {
    if (i % 2 === 1) {
      return (
        // biome-ignore lint/suspicious/noArrayIndexKey: parts are positional
        <mark key={i} className="bg-mneme-gold/30 text-mneme-gold rounded-sm px-0.5">
          {part}
        </mark>
      );
    }
    // biome-ignore lint/suspicious/noArrayIndexKey: parts are positional
    return <span key={i}>{part}</span>;
  });
}

function basenameOf(p: string): { folder: string; file: string } {
  const idx = p.lastIndexOf('/');
  if (idx === -1) return { folder: '', file: p };
  return { folder: p.slice(0, idx + 1), file: p.slice(idx + 1) };
}

function ResultRow({
  wellId,
  result,
  query,
}: {
  wellId: string;
  result: SearchResult;
  query: string;
}): React.JSX.Element {
  const openInActiveGroup = useWorkspaceStore((s) => s.openInActiveGroup);
  const setRevealTarget = useEditorStore((s) => s.setRevealTarget);
  const { folder, file } = basenameOf(result.path);
  const contentMatches = result.matches.filter((m) => m.kind === 'content');
  const hasFilenameMatch = result.matches.some((m) => m.kind === 'filename');

  return (
    <div
      className="flex w-full flex-col gap-0.5 border-b border-border px-3 py-2 text-left"
      title={result.path}
    >
      <button
        type="button"
        onClick={() => {
          openInActiveGroup({ wellId, path: result.path });
          setRevealTarget(null);
        }}
        className="flex w-full flex-col gap-0.5 text-left hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-mneme-cyan"
      >
        <div className="flex min-w-0 items-center gap-2 text-sm">
          <FileText className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
          <span className="min-w-0 flex-1 truncate">
            {hasFilenameMatch
              ? highlight(file.replace(/\.md$/, ''), query)
              : file.replace(/\.md$/, '')}
          </span>
        </div>
        {folder && (
          <span
            className="truncate pl-5 text-[10px] text-muted-foreground font-mono"
            title={folder}
          >
            {folder}
          </span>
        )}
      </button>
      {contentMatches.slice(0, 3).map((m, i) => (
        <button
          type="button"
          key={`${m.line}-${m.column}-${i}`}
          onClick={() => {
            openInActiveGroup({ wellId, path: result.path });
            setRevealTarget({
              docKey: docKeyOf(wellId, result.path),
              line: m.line,
              column: m.column,
            });
          }}
          className="flex w-full gap-2 pl-5 text-left text-xs text-muted-foreground hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-mneme-cyan"
        >
          <span className="shrink-0 font-mono text-mneme-text-muted">L{m.line}</span>
          <span className="min-w-0 truncate">{highlight(m.snippet, query)}</span>
        </button>
      ))}
      {contentMatches.length > 3 && (
        <span className="pl-5 text-[10px] italic text-muted-foreground">
          +{contentMatches.length - 3} more match{contentMatches.length - 3 === 1 ? '' : 'es'}
        </span>
      )}
    </div>
  );
}

export function SearchResults({ wellId, query, options }: Props): React.JSX.Element {
  const search = useSearch(wellId, query, true, options);

  if (query.trim().length < 2) {
    return (
      <div className="flex flex-col items-center gap-1 px-3 py-6 text-center text-xs text-muted-foreground">
        <Search className="h-6 w-6 opacity-30" />
        <span>Type at least 2 characters</span>
      </div>
    );
  }
  if (search.isLoading) {
    return <div className="px-3 py-4 text-xs text-muted-foreground">Searching…</div>;
  }
  if (search.error) {
    return (
      <div className="px-3 py-4 text-xs text-destructive">{(search.error as Error).message}</div>
    );
  }
  const results = search.data?.results ?? [];
  if (results.length === 0) {
    return (
      // biome-ignore lint/a11y/useSemanticElements: intentional ARIA live region; <output> implies form-output semantics that don't apply to a search-results announcement.
      <div
        role="status"
        aria-live="polite"
        className="flex flex-col items-center gap-1 px-3 py-6 text-center text-xs italic text-muted-foreground"
      >
        <span>No matches for "{query}"</span>
      </div>
    );
  }

  return (
    <div className="flex flex-col">
      {/* biome-ignore lint/a11y/useSemanticElements: intentional ARIA live region; <output> implies form-output semantics that don't apply to a search-results count. */}
      <div
        role="status"
        aria-live="polite"
        aria-atomic="true"
        className="border-b border-border bg-mneme-bg-2 px-3 py-1.5 text-[10px] uppercase tracking-wider text-muted-foreground"
      >
        {results.length} file{results.length === 1 ? '' : 's'} match
      </div>
      {results.map((r) => (
        <ResultRow key={r.path} wellId={wellId} result={r} query={query} />
      ))}
    </div>
  );
}
