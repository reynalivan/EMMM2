import ObjectListStates from './ObjectListStates';

interface ObjectListStateHostProps {
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

function formatErrorMessage(errorInfo: unknown): string | undefined {
  if (!errorInfo) {
    return undefined;
  }

  if (errorInfo instanceof Error) {
    return errorInfo.message;
  }

  if (typeof errorInfo === 'object') {
    return Object.values(errorInfo).join(': ');
  }

  return String(errorInfo);
}

export default function ObjectListStateHost({
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
}: ObjectListStateHostProps) {
  return (
    <ObjectListStates
      isLoading={isLoading}
      isError={isError}
      errorMessage={formatErrorMessage(errorInfo)}
      hasNoGame={hasNoGame}
      isEmpty={isEmpty}
      sidebarSearchQuery={sidebarSearchQuery}
      activeFilters={activeFilters}
      onClearFilters={onClearFilters}
      onClearSearch={onClearSearch}
      onCreateNew={onCreateNew}
      onAutoSetup={onAutoSetup}
    />
  );
}
