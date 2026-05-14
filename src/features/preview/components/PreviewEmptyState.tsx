import { FileArchive, FolderOpen, FolderPlus } from 'lucide-react';
import { useTranslation } from 'react-i18next';

interface PreviewEmptyStateProps {
  sourceUnavailableMessage: string | null;
  onImportArchives: () => void;
  onImportFolders: () => void;
}

export default function PreviewEmptyState({
  sourceUnavailableMessage,
  onImportArchives,
  onImportFolders,
}: PreviewEmptyStateProps) {
  const { t } = useTranslation(['preview']);
  const mutationsDisabled = Boolean(sourceUnavailableMessage);

  return (
    <div className="mx-auto flex h-full w-full max-w-140 flex-col items-center justify-center p-6 text-center border-l border-base-content/5 bg-base-100/30 backdrop-blur-md">
      <div className="mb-6 text-base-content/50">
        {sourceUnavailableMessage && (
          <div className="mb-4 rounded border border-warning/30 bg-warning/10 px-3 py-2 text-xs text-warning">
            {sourceUnavailableMessage}
          </div>
        )}
        <FolderOpen size={48} className="mx-auto mb-4 opacity-50" />
        <p className="text-xl font-bold text-base-content mb-2">
          {t('preview:empty.no_mod_selected')}
        </p>
        <p className="text-sm">{t('preview:empty.select_folder')}</p>
      </div>
      <div className="flex w-full max-w-xs flex-col gap-3">
        <button
          className="btn btn-outline btn-primary gap-2"
          disabled={mutationsDisabled}
          onClick={onImportArchives}
        >
          <FileArchive size={18} />
          {t('preview:actions.import_archives')}
        </button>
        <button
          className="btn btn-outline btn-primary gap-2"
          disabled={mutationsDisabled}
          onClick={onImportFolders}
        >
          <FolderPlus size={18} />
          {t('preview:actions.import_folders')}
        </button>
      </div>
    </div>
  );
}
