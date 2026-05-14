import { Loader2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
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

  if (isLoading) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <Loader2 size={24} className="animate-spin text-primary/50" />
      </div>
    );
  }

  if (isError) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center gap-2 p-4">
        <p className="text-xs text-error/60">
          {error instanceof Error ? error.message : String(error) || t('status.load_error')}
        </p>
      </div>
    );
  }

  if (visibleCount > 0) {
    return null;
  }

  if (isFlatModRoot) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center gap-2 p-6 text-base-content/40">
        <p className="text-sm font-medium">{t('status.no_subfolders')}</p>
        <p className="text-xs text-center">{t('status.preview_hint')}</p>
      </div>
    );
  }

  return (
    <FolderGridEmpty
      explorerSearchQuery={explorerSearchQuery}
      currentPath={currentPath}
      setExplorerSearch={setExplorerSearch}
      handleBreadcrumbClick={handleBreadcrumbClick}
      handleImportFiles={(paths) => {
        void handleImportFiles(paths);
      }}
    />
  );
}
