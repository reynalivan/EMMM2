/**
 * ObjectListStates — loading, error, empty, and no-game placeholder states.
 * Extracted from ObjectList for modularity (350-line limit).
 */

import { Loader2, AlertCircle, FolderOpen, FolderPlus } from 'lucide-react';
import { useTranslation } from 'react-i18next';

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
  onCreateNew: () => void;
  onAutoSetup: () => void;
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
  onCreateNew,
  onAutoSetup,
}: StatesProps) {
  const { t } = useTranslation(['objects']);

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
          {errorMessage ?? t('states.load_error')}
        </p>
      </div>
    );
  }

  if (hasNoGame) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center gap-3 p-6">
        <FolderOpen size={40} className="text-base-content/15" />
        <p className="text-sm text-base-content/40 text-center">{t('states.select_game')}</p>
      </div>
    );
  }

  if (isEmpty) {
    const hasActiveFilters = Object.values(activeFilters).some((v) => v.length > 0);
    const message = sidebarSearchQuery
      ? t('states.no_search_results')
      : hasActiveFilters
        ? t('states.no_filter_results')
        : t('states.empty_hint');

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
            {t('states.clear_filters')}
          </button>
        )}
        {sidebarSearchQuery && (
          <button
            className="btn btn-sm btn-ghost gap-2 text-primary mt-1"
            onClick={onClearSearch}
            data-testid="clear-search-btn"
          >
            {t('states.clear_search')}
          </button>
        )}
        {!sidebarSearchQuery && !hasActiveFilters && (
          <div className="flex flex-col gap-2 mt-4 w-full max-w-50 items-center">
            <button className="btn btn-outline w-full gap-2" onClick={onCreateNew}>
              <FolderPlus size={16} />
              {t('states.add_folder')}
            </button>
            <button className="btn btn-primary w-full gap-2" onClick={onAutoSetup}>
              <FolderPlus size={16} />
              {t('states.auto_setup')}
            </button>
          </div>
        )}
      </div>
    );
  }

  return null;
}
