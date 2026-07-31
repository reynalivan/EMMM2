/**
 * ObjectListStates — loading, error, empty, and no-game placeholder states.
 * Loading/error scaffolding lives in the shared ListStateView.
 */

import { FolderPlus } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import ListStateView, { ListEmptyState } from '../../../components/ui/ListStateView';

interface StatesProps {
  isLoading: boolean;
  isError: boolean;
  errorInfo: unknown;
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
  errorInfo,
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

  const renderEmpty = () => {
    if (hasNoGame) {
      return <ListEmptyState message={t('states.select_game')} />;
    }

    if (!isEmpty) {
      return null;
    }

    const hasActiveFilters = Object.values(activeFilters).some((v) => v.length > 0);
    const message = sidebarSearchQuery
      ? t('states.no_search_results')
      : hasActiveFilters
        ? t('states.no_filter_results')
        : t('states.empty_hint');

    return (
      <ListEmptyState message={message} testId="empty-state">
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
      </ListEmptyState>
    );
  };

  return (
    <ListStateView
      isLoading={isLoading}
      isError={isError}
      error={errorInfo}
      errorFallback={t('states.load_error')}
    >
      {renderEmpty()}
    </ListStateView>
  );
}
