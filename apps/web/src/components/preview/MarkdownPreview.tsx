import type React from 'react';
import { useEffect, useMemo, useRef } from 'react';
import { decodeWikilinkTarget, isWikilinkAnchor, renderMarkdown } from '../../lib/markdown.ts';
import { usePrefs } from '../../lib/settings.ts';
import { useTheme } from '../../lib/theme.ts';
import { initMermaid, renderMermaidIn } from './mermaid-init.ts';
import './preview.css';

type Props = {
  source: string;
  onWikilinkClick?: (target: string) => void;
};

export function MarkdownPreview({ source, onWikilinkClick }: Props): React.JSX.Element {
  const ref = useRef<HTMLDivElement>(null);
  const { effectiveMode } = useTheme();
  const { previewMermaid } = usePrefs();
  const html = useMemo(() => renderMarkdown(source), [source]);

  useEffect(() => {
    initMermaid(effectiveMode);
  }, [effectiveMode]);

  // biome-ignore lint/correctness/useExhaustiveDependencies: html change is the trigger to re-render mermaid diagrams
  useEffect(() => {
    if (!ref.current) return;
    if (previewMermaid) void renderMermaidIn(ref.current);
  }, [html, previewMermaid]);

  useEffect(() => {
    if (!ref.current) return;
    const root = ref.current;
    const handler = (e: MouseEvent) => {
      const a = (e.target as HTMLElement | null)?.closest('a');
      if (!a) return;
      const href = a.getAttribute('href') ?? '';
      if (isWikilinkAnchor(href)) {
        e.preventDefault();
        const target = decodeWikilinkTarget(href);
        if (target && onWikilinkClick) onWikilinkClick(target);
      } else if (href.startsWith('http')) {
        a.setAttribute('target', '_blank');
        a.setAttribute('rel', 'noopener noreferrer');
      }
    };
    root.addEventListener('click', handler);
    return () => root.removeEventListener('click', handler);
  }, [onWikilinkClick]);

  // The HTML below is DOMPurify-sanitized by renderMarkdown — safe to inject.
  // (biome-ignore lint/security/noDangerouslySetInnerHtml on the line below.)
  return (
    <div className="h-full overflow-auto bg-background">
      <article
        ref={ref}
        className="mneme-preview"
        // biome-ignore lint/security/noDangerouslySetInnerHtml: html is DOMPurify-sanitized in renderMarkdown
        dangerouslySetInnerHTML={{ __html: html }}
      />
    </div>
  );
}
