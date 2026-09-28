import DOMPurify from 'isomorphic-dompurify';
import mermaid from 'mermaid';

let initialized = false;

// Mermaid's securityLevel: 'strict' already sanitizes its own SVG output,
// but that's a single point of trust: diagram source here can originate
// from more than direct typing (the MCP server, a synced/shared Well from
// another device), and a future Mermaid version could regress that
// internal sanitization without Mneme knowing. This config only allows SVG
// structure/presentation — no script-bearing tags or event-handler attrs.
const MERMAID_SVG_PURIFY_CONFIG = {
  USE_PROFILES: { svg: true, svgFilters: true },
  ADD_TAGS: [] as string[],
  FORBID_TAGS: ['script'],
  FORBID_ATTR: ['onerror', 'onload', 'onclick', 'onmouseover', 'onfocus', 'onanimationstart'],
};

export function initMermaid(theme: 'dark' | 'light' = 'dark'): void {
  if (initialized) return;
  mermaid.initialize({
    startOnLoad: false,
    securityLevel: 'strict',
    theme: 'base',
    themeVariables: {
      darkMode: theme === 'dark',
      background: theme === 'dark' ? '#0B1929' : '#F8FAFC',
      primaryColor: '#1E293B',
      primaryTextColor: '#E2E8F0',
      primaryBorderColor: '#4FD1C5',
      lineColor: '#4FD1C5',
      secondaryColor: '#142847',
      tertiaryColor: '#283449',
      mainBkg: '#1E293B',
      noteBkgColor: '#D4A24E',
      noteTextColor: '#0B1929',
    },
  });
  initialized = true;
}

export async function renderMermaidIn(root: HTMLElement): Promise<void> {
  const nodes = root.querySelectorAll<HTMLElement>('pre > code.language-mermaid');
  for (let i = 0; i < nodes.length; i++) {
    const codeEl = nodes[i];
    if (!codeEl) continue;
    const pre = codeEl.parentElement;
    if (!pre) continue;
    const id = `mermaid-${Date.now()}-${i}`;
    const source = codeEl.textContent ?? '';
    try {
      const { svg } = await mermaid.render(id, source);
      const container = document.createElement('div');
      container.className = 'mermaid-rendered';
      // Defense-in-depth: sanitize on top of Mermaid's own strict mode
      // before this touches the DOM — see MERMAID_SVG_PURIFY_CONFIG above.
      container.innerHTML = DOMPurify.sanitize(svg, MERMAID_SVG_PURIFY_CONFIG) as unknown as string;
      pre.replaceWith(container);
    } catch (err) {
      const errDiv = document.createElement('div');
      errDiv.className = 'mermaid-error';
      errDiv.textContent = `Mermaid render failed: ${err instanceof Error ? err.message : String(err)}`;
      pre.replaceWith(errDiv);
    }
  }
}
