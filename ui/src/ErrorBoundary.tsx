// ui/src/ErrorBoundary.tsx — React error boundary for production resilience
//
// Catches render errors in the component tree and shows a fallback UI
// instead of a blank white screen. Critical for Tauri desktop builds.

import { Component, type ErrorInfo, type ReactNode } from 'react';

interface Props {
  children: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

export default class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    // ponytail: console.error is the right level here — this is a crash
    console.error('[Aurora] UI crash:', error.message);
    console.error('[Aurora] Component stack:', info.componentStack);
  }

  render() {
    if (this.state.hasError) {
      return (
        <div style={{
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          height: '100vh',
          background: '#0a0a1a',
          color: '#c8d6e5',
          fontFamily: 'system-ui, sans-serif',
          gap: '1rem',
          padding: '2rem',
          textAlign: 'center',
        }}>
          <div style={{ fontSize: '3rem' }}>⚠</div>
          <h1 style={{ fontSize: '1.25rem', fontWeight: 600, margin: 0 }}>
            Something went wrong
          </h1>
          <p style={{ fontSize: '0.875rem', color: '#8395a7', maxWidth: '32rem', margin: 0 }}>
            {this.state.error?.message || 'An unexpected error occurred.'}
          </p>
          <button
            onClick={() => {
              this.setState({ hasError: false, error: null });
              window.location.reload();
            }}
            style={{
              marginTop: '0.5rem',
              padding: '0.5rem 1.25rem',
              border: '1px solid #5f27cd',
              borderRadius: '6px',
              background: 'transparent',
              color: '#c8d6e5',
              cursor: 'pointer',
              fontSize: '0.875rem',
            }}
          >
            Reload
          </button>
        </div>
      );
    }

    return this.props.children;
  }
}
