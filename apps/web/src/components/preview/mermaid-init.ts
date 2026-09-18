import mermaid from 'mermaid';

let initialized = false;

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
      // svg comes from mermaid's own renderer — not user-controlled HTML
      container.innerHTML = svg; // noqa: security — mermaid SVG output
      pre.replaceWith(container);
    } catch (err) {
      const errDiv = document.createElement('div');
      errDiv.className = 'mermaid-error';
      errDiv.textContent = `Mermaid render failed: ${err instanceof Error ? err.message : String(err)}`;
      pre.replaceWith(errDiv);
    }
  }
}
