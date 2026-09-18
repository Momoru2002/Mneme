import type { Frontmatter } from '@mneme/shared';
import { ChevronDown, ChevronRight, ExternalLink } from 'lucide-react';
import type React from 'react';
import { useState } from 'react';

const STORAGE_KEY = 'mneme.frontmatter.expanded';

const URL_RE = /^https?:\/\//;
const ISO_DATE_RE = /^\d{4}-\d{2}-\d{2}/;

function parseTags(value: unknown): string[] {
  if (Array.isArray(value)) return value.map((v) => String(v));
  if (typeof value === 'string')
    return value
      .split(',')
      .map((s) => s.trim())
      .filter(Boolean);
  return [];
}

function stringOf(value: unknown): string | null {
  if (typeof value === 'string' && value.trim().length > 0) return value;
  if (typeof value === 'number') return String(value);
  return null;
}

function formatIsoDate(s: string): string {
  // Show "2026-05-16" → "May 16, 2026". Fall back to raw.
  const date = new Date(s);
  if (Number.isNaN(date.getTime())) return s;
  return date.toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' });
}

function FieldValue({ field, value }: { field: string; value: unknown }): React.JSX.Element {
  if (Array.isArray(value)) {
    return (
      <div className="flex flex-wrap gap-1">
        {value.map((v, i) => (
          <span
            key={`${i}-${String(v)}`}
            className="rounded-sm bg-mneme-cyan/15 px-1.5 py-0.5 text-mneme-cyan"
          >
            {String(v)}
          </span>
        ))}
      </div>
    );
  }
  if (typeof value === 'string') {
    if (field === 'link' || field === 'url' || URL_RE.test(value)) {
      return (
        <a
          href={value}
          target="_blank"
          rel="noopener noreferrer"
          className="inline-flex items-center gap-1 break-all text-mneme-info underline-offset-2 hover:text-mneme-gold hover:underline"
        >
          <ExternalLink className="h-3 w-3 shrink-0" />
          <span className="break-all">{value}</span>
        </a>
      );
    }
    if (ISO_DATE_RE.test(value)) {
      return (
        <span className="font-mono" title={value}>
          {formatIsoDate(value)}
        </span>
      );
    }
    return <span className="break-words">{value}</span>;
  }
  if (typeof value === 'boolean') {
    return (
      <span className={value ? 'text-mneme-success' : 'text-mneme-text-dim'}>
        {value ? 'true' : 'false'}
      </span>
    );
  }
  if (value === null) return <span className="text-mneme-text-dim italic">null</span>;
  if (typeof value === 'object')
    return <span className="font-mono text-xs">{JSON.stringify(value)}</span>;
  return <span className="break-words">{String(value)}</span>;
}

function FieldRow({ field, value }: { field: string; value: unknown }): React.JSX.Element {
  return (
    <>
      <dt className="text-muted-foreground">{field}</dt>
      <dd className="min-w-0 text-foreground">
        <FieldValue field={field} value={value} />
      </dd>
    </>
  );
}

type Props = {
  data: Frontmatter;
};

export function FrontmatterPanel({ data }: Props): React.JSX.Element | null {
  const entries = Object.entries(data);
  const [expanded, setExpanded] = useState<boolean>(
    () => typeof window !== 'undefined' && window.localStorage.getItem(STORAGE_KEY) === 'true',
  );
  if (entries.length === 0) return null;

  const tags = parseTags(data.tags);
  const linkRaw =
    typeof data.link === 'string' ? data.link : typeof data.url === 'string' ? data.url : null;
  const typeStr = stringOf(data.type) ?? stringOf(data.tipe) ?? stringOf(data.kind);
  const durationStr = stringOf(data.duration) ?? stringOf(data.durasi);

  const toggle = () => {
    const next = !expanded;
    setExpanded(next);
    try {
      window.localStorage.setItem(STORAGE_KEY, String(next));
    } catch {
      // ignore (private mode, etc.)
    }
  };

  return (
    <section className="border-b border-mneme-gold/30 bg-mneme-gold/5">
      <div className="flex min-w-0 items-center gap-2 px-3 py-1.5">
        <button
          type="button"
          onClick={toggle}
          className="flex shrink-0 items-center gap-1.5 rounded-sm px-1.5 py-0.5 text-xs font-semibold uppercase tracking-wider text-mneme-gold hover:bg-mneme-gold/15"
          aria-expanded={expanded}
        >
          {expanded ? (
            <ChevronDown className="h-3.5 w-3.5" />
          ) : (
            <ChevronRight className="h-3.5 w-3.5" />
          )}
          Properties
        </button>

        {!expanded && (
          <div className="flex min-w-0 flex-1 items-center gap-1.5 overflow-hidden text-xs text-muted-foreground">
            {typeStr && (
              <span className="shrink-0 rounded-sm border border-mneme-border bg-mneme-bg-2 px-1.5 py-0.5 text-mneme-text">
                {typeStr}
              </span>
            )}
            {durationStr && (
              <span className="shrink-0 rounded-sm border border-mneme-border bg-mneme-bg-2 px-1.5 py-0.5">
                {durationStr}
              </span>
            )}
            {tags.length > 0 && (
              <div className="flex min-w-0 items-center gap-1 overflow-hidden">
                {tags.slice(0, 5).map((t) => (
                  <span
                    key={t}
                    className="shrink-0 rounded-sm bg-mneme-cyan/15 px-1.5 py-0.5 text-mneme-cyan"
                  >
                    {t}
                  </span>
                ))}
                {tags.length > 5 && (
                  <span className="shrink-0 text-mneme-text-dim">+{tags.length - 5}</span>
                )}
              </div>
            )}
          </div>
        )}

        <span className="ml-auto shrink-0 text-[10px] uppercase tracking-wider text-muted-foreground">
          {entries.length} {entries.length === 1 ? 'field' : 'fields'}
        </span>
        {linkRaw && (
          <a
            href={linkRaw}
            target="_blank"
            rel="noopener noreferrer"
            onClick={(e) => e.stopPropagation()}
            className="shrink-0 rounded-sm p-1 text-mneme-info hover:bg-mneme-gold/15 hover:text-mneme-gold"
            title={`Open link: ${linkRaw}`}
            aria-label="Open external link"
          >
            <ExternalLink className="h-3.5 w-3.5" />
          </a>
        )}
      </div>

      {expanded && (
        <div className="px-4 pb-3 pt-1">
          <dl className="grid grid-cols-[140px_1fr] gap-x-3 gap-y-1.5 text-xs">
            {entries.map(([k, v]) => (
              <FieldRow key={k} field={k} value={v} />
            ))}
          </dl>
        </div>
      )}
    </section>
  );
}
