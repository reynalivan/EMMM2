import { useTranslation } from 'react-i18next';
import ListStateView from '../../../components/ui/ListStateView';
import FolderGridEmpty from './FolderGridEmpty';

interface FolderGridStateViewsProps {
  isLoading: boolean;
  isError: boolean;
  error: unknown;
  visibleCount: number;
  isFlatModRoot: boolean;
  explorerSearchQuery: string;
  currentPath: string[];
  setExplorerSearch: (query: string) => void;
  handleBreadcrumbClick: (index: number) => void;
  handleImportFiles: (paths: string[]) => void | Promise<void>;
}

export default function FolderGridStateViews({
  isLoading,
  isError,
  error,
  visibleCount,
  isFlatModRoot,
  explorerSearchQuery,
  currentPath,
  setExplorerSearch,
  handleBreadcrumbClick,
  handleImportFiles,
}: FolderGridStateViewsProps) {
  const { t } = useTranslation(['grid']);

  return (
    <ListStateView
      isLoading={isLoading}
      isError={isError}
      error={error}
      errorFallback={t('status.load_error')}
    >
      {visibleCount > 0 ? null : isFlatModRoot ? (
        <div className="flex-1 flex flex-col items-center justify-center gap-2 p-6 text-base-content/40">
          <p className="text-sm font-medium">{t('status.no_subfolders')}</p>
          <p className="text-xs text-center">{t('status.preview_hint')}</p>
        </div>
      ) : (
        <FolderGridEmpty
          explorerSearchQuery={explorerSearchQuery}
          currentPath={currentPath}
          setExplorerSearch={setExplorerSearch}
          handleBreadcrumbClick={handleBreadcrumbClick}
          handleImportFiles={(paths) => {
            void handleImportFiles(paths);
          }}
        />
      )}
    </ListStateView>
  );
}
