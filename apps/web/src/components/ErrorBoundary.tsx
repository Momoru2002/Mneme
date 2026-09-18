import { Component, type ReactNode } from 'react';

type Props = { children: ReactNode };
type State = { error: Error | null };

export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: { componentStack?: string }): void {
    // eslint-disable-next-line no-console
    console.error('[Mneme] Uncaught error', error, info);
  }

  render(): ReactNode {
    if (this.state.error) {
      return (
        <div className="flex min-h-screen items-center justify-center bg-mneme-bg p-6 text-mneme-text">
          <div className="max-w-md flex flex-col gap-4 rounded-lg border border-mneme-danger/40 bg-mneme-surface p-6">
            <h1 className="text-xl font-semibold text-mneme-danger">Something went wrong</h1>
            <p className="text-sm text-mneme-text-muted">
              An unexpected error occurred. Reload the page to recover. The full error has been
              logged to the browser console.
            </p>
            <pre className="overflow-auto rounded-md border border-mneme-border bg-mneme-bg-2 p-3 text-xs text-mneme-warning">
              {this.state.error.message}
            </pre>
            <button
              type="button"
              onClick={() => window.location.reload()}
              className="rounded-md bg-mneme-cyan px-4 py-2 text-sm font-semibold text-mneme-bg hover:brightness-110"
            >
              Reload
            </button>
          </div>
        </div>
      );
    }
    return this.props.children;
  }
}
