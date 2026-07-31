import type { ReactNode } from 'react';
import { AlertCircle, FolderOpen, Loader2 } from 'lucide-react';

function formatErrorMessage(error: unknown): string | undefined {
  if (!error) {
    return undefined;
  }

  if (error instanceof Error) {
    return error.message;
  }

  if (typeof error === 'object') {
    return Object.values(error).join(': ');
  }

  return String(error);
}

/** Centered empty placeholder shared by list panels (FolderOpen icon + message + actions). */
export function ListEmptyState({
  message,
  testId,
  children,
}: {
  message: string;
  testId?: string;
  children?: ReactNode;
}) {
  return (
    <div
      className="flex-1 flex flex-col items-center justify-center gap-3 p-6"
      data-testid={testId}
    >
      <FolderOpen size={40} className="text-base-content/15" />
      <p className="text-sm text-base-content/40 text-center">{message}</p>
      {children}
    </div>
  );
}

interface ListStateViewProps {
  isLoading: boolean;
  isError: boolean;
  error: unknown;
  /** Shown when the error carries no usable message. */
  errorFallback: string;
  /** Rendered when neither loading nor error (typically the empty-state branch). */
  children?: ReactNode;
}

/**
 * ListStateView — shared loading/error scaffolding for list panels
 * (folder grid and object list). Empty-state decisions stay with the caller.
 */
export default function ListStateView({
  isLoading,
  isError,
  error,
  errorFallback,
  children,
}: ListStateViewProps) {
  if (isLoading) {
    return (
      <div className="flex-1 flex items-center justify-center" data-testid="loading-spinner">
        <Loader2 size={24} className="animate-spin text-primary/50" />
      </div>
    );
  }

  if (isError) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center gap-2 p-4">
        <AlertCircle size={24} className="text-error/50" />
        <p className="text-xs text-base-content/50 text-center">
          {formatErrorMessage(error) ?? errorFallback}
        </p>
      </div>
    );
  }

  return <>{children}</>;
}
