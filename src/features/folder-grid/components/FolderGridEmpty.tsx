import { FolderOpen, ChevronLeft, Upload, FolderInput } from 'lucide-react';
import { open } from '@tauri-apps/plugin-dialog';
import { useTranslation } from 'react-i18next';

export interface FolderGridEmptyProps {
  explorerSearchQuery: string;
  currentPath: string[];
  setExplorerSearch: (query: string) => void;
  handleBreadcrumbClick: (index: number) => void;
  handleImportFiles: (paths: string[]) => void;
}

export default function FolderGridEmpty({
  explorerSearchQuery,
  currentPath,
  setExplorerSearch,
  handleBreadcrumbClick,
  handleImportFiles,
}: FolderGridEmptyProps) {
  const { t } = useTranslation(['grid']);

  return (
    <div className="flex-1 flex flex-col items-center justify-center gap-4 p-6">
      <FolderOpen size={40} className="text-base-content/15" />
      <p className="text-sm text-base-content/40 text-center">
        {explorerSearchQuery
          ? t('empty.no_match')
          : currentPath.length > 0
            ? t('empty.folder_empty')
            : t('empty.no_mods')}
      </p>

      {explorerSearchQuery && (
        <button
          className="btn btn-sm btn-ghost gap-2 text-primary mt-2"
          onClick={() => setExplorerSearch('')}
        >
          {t('empty.clear_search')}
        </button>
      )}

      {/* When navigated into an empty folder → show back + import suggestion */}
      {!explorerSearchQuery && currentPath.length > 0 && (
        <div className="flex flex-col items-center gap-3 mt-2">
          <button
            className="btn btn-ghost btn-sm gap-2 text-base-content/60"
            onClick={() => handleBreadcrumbClick(currentPath.length - 2)}
          >
            <ChevronLeft size={16} />
            {t('empty.go_back')}
          </button>

          <div className="divider text-base-content/20 text-[10px] my-0">
            {t('empty.divider_or')}
          </div>

          <p className="text-xs text-base-content/30">{t('empty.import_hint')}</p>
          <div className="flex gap-2">
            <button
              className="btn btn-outline btn-sm gap-2"
              onClick={async () => {
                const selected = await open({
                  multiple: true,
                  filters: [{ name: 'Archives', extensions: ['zip', 'rar', '7z'] }],
                });
                if (selected) {
                  const paths = Array.isArray(selected) ? selected : [selected];
                  handleImportFiles(paths);
                }
              }}
            >
              <Upload size={14} />
              {t('empty.import_archive')}
            </button>
            <button
              className="btn btn-outline btn-sm gap-2"
              onClick={async () => {
                const selected = await open({ directory: true, multiple: false });
                if (selected) {
                  const paths = Array.isArray(selected) ? selected : [selected];
                  handleImportFiles(paths);
                }
              }}
            >
              <FolderInput size={14} />
              {t('empty.import_folder')}
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
