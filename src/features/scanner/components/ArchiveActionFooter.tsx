import { useTranslation } from 'react-i18next';

interface ArchiveActionFooterProps {
  isExtracting: boolean;
  selectedCount: number;
  hasValidationErrors: boolean;
  extractProgress?: { current: number; total: number } | null;
  fileProgress?: { fileName: string; fileIndex: number; totalFiles: number } | null;
  onSkip: () => void;
  onExtract: () => void;
  onRequestStop: () => void;
}

export default function ArchiveActionFooter({
  isExtracting,
  selectedCount,
  hasValidationErrors,
  extractProgress,
  fileProgress,
  onSkip,
  onExtract,
  onRequestStop,
}: ArchiveActionFooterProps) {
  const { t } = useTranslation(['scanner']);

  return (
    <div className="modal-action bg-base-200 p-4 m-0 flex flex-col gap-2 border-t border-base-300 shrink-0">
      {isExtracting && extractProgress && extractProgress.total > 0 && (
        <div className="flex flex-col gap-1 w-full">
          <div className="flex justify-between text-xs text-base-content/60">
            <span>
              {t('extract.progress_archive', {
                current: extractProgress.current,
                total: extractProgress.total,
              })}
            </span>
            <span>{Math.round((extractProgress.current / extractProgress.total) * 100)}%</span>
          </div>
          <progress
            className="progress progress-primary w-full"
            value={extractProgress.current}
            max={extractProgress.total}
          />
          {fileProgress && (
            <div className="flex flex-col gap-0.5 mt-1">
              <div className="flex justify-between text-[10px] text-base-content/40">
                <span className="truncate max-w-70" title={fileProgress.fileName}>
                  {fileProgress.fileName || t('extract.progress_extracting')}
                </span>
                {fileProgress.totalFiles > 0 && (
                  <span>
                    {t('extract.progress_files', {
                      current: fileProgress.fileIndex,
                      total: fileProgress.totalFiles,
                    })}
                  </span>
                )}
              </div>
              {fileProgress.totalFiles > 0 && (
                <progress
                  className="progress progress-accent progress-xs w-full opacity-60"
                  value={fileProgress.fileIndex}
                  max={fileProgress.totalFiles}
                />
              )}
            </div>
          )}
        </div>
      )}
      <div className="flex justify-between items-center w-full">
        <button className="btn btn-ghost btn-sm" onClick={onSkip} disabled={isExtracting}>
          {t('extract.action_skip')}
        </button>
        <div className="flex gap-2 items-center">
          <div className="text-xs text-base-content/50 mr-2">
            {t('extract.selected_count', { count: selectedCount })}
          </div>

          {isExtracting ? (
            <button className="btn btn-error btn-sm" onClick={onRequestStop}>
              {t('extract.action_stop')}
            </button>
          ) : (
            <button
              className="btn btn-primary btn-sm"
              onClick={onExtract}
              disabled={selectedCount === 0 || hasValidationErrors}
            >
              {t('extract.action_extract')}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
