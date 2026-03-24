import { Component, ErrorInfo, ReactNode } from 'react';
import { AlertTriangle } from 'lucide-react';
import i18n from '../../lib/i18n';

interface Props {
  children?: ReactNode;
  fallback?: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
  errorInfo: ErrorInfo | null;
}

export class ErrorBoundary extends Component<Props, State> {
  public state: State = {
    hasError: false,
    error: null,
    errorInfo: null,
  };

  public static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error, errorInfo: null };
  }

  public componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error('Uncaught error:', error, errorInfo);
    this.setState({ errorInfo });
  }

  public render() {
    if (this.state.hasError) {
      if (this.props.fallback) return this.props.fallback;

      return (
        <div className="flex flex-col items-center justify-center p-8 h-full bg-base-100 text-error overflow-auto">
          <AlertTriangle size={48} className="mb-4" />
          <h2 className="text-2xl font-bold mb-2">{i18n.t('common:errors.boundary_title')}</h2>
          <p className="text-base-content/70 mb-4">
            {this.state.error?.message || i18n.t('common:errors.unexpected')}
          </p>
          <div className="bg-base-300 p-4 rounded-lg w-full max-w-2xl overflow-x-auto text-left">
            <pre className="text-xs font-mono whitespace-pre-wrap text-base-content/80">
              {this.state.errorInfo?.componentStack || this.state.error?.stack}
            </pre>
          </div>
          <button className="btn btn-primary mt-6" onClick={() => window.location.reload()}>
            {i18n.t('common:actions.reload_app')}
          </button>
        </div>
      );
    }

    return this.props.children;
  }
}
