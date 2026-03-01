/**
 * ObjectListStates — loading, error, empty, and no-game placeholder states.
 * Extracted from ObjectList for modularity (350-line limit).
 */

import { Loader2, AlertCircle, FolderOpen, FolderPlus } from 'lucide-react';

interface StatesProps {
  isLoading: boolean;
  isError: boolean;
  errorMessage: string | undefined;
  hasNoGame: boolean;
  isEmpty: boolean;
  sidebarSearchQuery: string;
  activeFilters: Record<string, string[]>;
  onClearFilters: () => void;
  onClearSearch: () => void;
  isSyncing: boolean;
  onSync: () => void;
}

export default function ObjectListStates({
  isLoading,
  isError,
  errorMessage,
  hasNoGame,
  isEmpty,
  sidebarSearchQuery,
  activeFilters,
  onClearFilters,
  onClearSearch,
  isSyncing,
  onSync,
}: StatesProps) {
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
          {errorMessage ?? 'Failed to load data'}
        </p>
      </div>
    );
  }

  if (hasNoGame) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center gap-3 p-6">
        <FolderOpen size={40} className="text-base-content/15" />
        <p className="text-sm text-base-content/40 text-center">
          Select a game from the top bar to get started
        </p>
      </div>
    );
  }

  if (isEmpty) {
    const hasActiveFilters = Object.values(activeFilters).some((v) => v.length > 0);
    const message = sidebarSearchQuery
      ? 'No results match your search'
      : hasActiveFilters
        ? 'No objects match filter'
        : 'Drag mod folders here or create a new object';

    return (
      <div
        className="flex-1 flex flex-col items-center justify-center gap-3 p-6"
        data-testid="empty-state"
      >
        <FolderOpen size={40} className="text-base-content/15" />
        <p className="text-sm text-base-content/40 text-center">{message}</p>
        {hasActiveFilters && (
          <button
            className="btn btn-sm btn-ghost gap-2 text-primary"
            onClick={onClearFilters}
            data-testid="clear-filters-btn"
          >
            Clear Filters
          </button>
        )}
        {sidebarSearchQuery && (
          <button
            className="btn btn-sm btn-ghost gap-2 text-primary mt-1"
            onClick={onClearSearch}
            data-testid="clear-search-btn"
          >
            Clear Search
          </button>
        )}
        {!sidebarSearchQuery && !hasActiveFilters && (
          <button
            className="btn btn-sm btn-outline btn-primary gap-2 mt-1"
            onClick={onSync}
            disabled={isSyncing}
          >
            <FolderPlus size={14} />
            {isSyncing ? 'Scanning...' : 'Scan & Create Objects'}
          </button>
        )}
      </div>
    );
  }

  return null;
}
