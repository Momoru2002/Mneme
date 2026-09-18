import DOMPurify from 'isomorphic-dompurify';
import { Marked } from 'marked';

const marked = new Marked({
  gfm: true,
  breaks: false,
});

// Pre-process [[Wikilinks]] before marked sees them.
// [[X|Y]] -> [Y](#wikilink-X) and [[X]] -> [X](#wikilink-X).
// Skips fenced code blocks (```...```) and inline code (`...`) so
// mermaid syntax like A[[subroutine]] isn't mangled.
function preprocessWikilinks(src: string): string {
  const blocks: string[] = [];
  const PLACEHOLDER = '\x00MNEME_CODE_';

  // Stash fenced blocks first, then inline code spans.
  const stashed = src
    .replace(/```[\s\S]*?```/g, (match) => {
      const idx = blocks.length;
      blocks.push(match);
      return `${PLACEHOLDER}${idx}\x00`;
    })
    .replace(/`[^`\n]+`/g, (match) => {
      const idx = blocks.length;
      blocks.push(match);
      return `${PLACEHOLDER}${idx}\x00`;
    });

  const rewritten = stashed.replace(
    /\[\[([^\]|]+)(?:\|([^\]]+))?\]\]/g,
    (_m, target: string, label?: string) => {
      const display = (label ?? target).trim();
      const slug = encodeURIComponent(target.trim());
      return `[${display}](#wikilink-${slug})`;
    },
  );

  return rewritten.replace(
    new RegExp(`${PLACEHOLDER}(\\d+)\\x00`, 'g'),
    (_full, idx: string) => blocks[Number(idx)] ?? '',
  );
}

const PURIFY_CONFIG = {
  ADD_ATTR: ['target'],
  ADD_TAGS: [] as string[],
  ALLOWED_URI_REGEXP:
    /^(?:(?:https?|mailto|tel|callto|sms|cid|xmpp|ftp|file|data|blob):|[^a-z]|[a-z+.-]+(?:[^a-z+.-:]|$)|#)/i,
};

export function renderMarkdown(src: string): string {
  const preprocessed = preprocessWikilinks(src);
  const rawHtml = marked.parse(preprocessed, { async: false }) as string;
  return DOMPurify.sanitize(rawHtml, PURIFY_CONFIG) as unknown as string;
}

export function isWikilinkAnchor(href: string): boolean {
  return href.startsWith('#wikilink-');
}

export function decodeWikilinkTarget(href: string): string | null {
  if (!isWikilinkAnchor(href)) return null;
  try {
    return decodeURIComponent(href.slice('#wikilink-'.length));
  } catch {
    return null;
  }
}
