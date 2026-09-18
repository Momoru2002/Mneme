import { useEffect } from 'react';

const SUFFIX = 'Mneme';

/**
 * Set the document title for the current route (WCAG 2.4.2 Page Titled).
 * Pass the page-specific part; the app name is appended. Pass `null` to show
 * just the app name. Restores nothing on unmount — the next route sets its own.
 */
export function useDocumentTitle(title: string | null): void {
  useEffect(() => {
    document.title = title ? `${title} — ${SUFFIX}` : SUFFIX;
  }, [title]);
}
