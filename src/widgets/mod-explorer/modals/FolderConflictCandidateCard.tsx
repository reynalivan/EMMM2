import { Pencil, Trash2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type {
  FolderConflictSummary,
  FolderNameConflictCandidate,
} from '../../../shared/api/tauri/bindings';
import { formatBytes } from '../../../shared/lib/utils/formatters';

interface Props {
  candidate: FolderNameConflictCandidate;
  detail?: FolderConflictSummary;
  isKeep: boolean;
  value: string;
  error?: string;
  disabled?: boolean;
  inputRef: (element: HTMLInputElement | null) => void;
  onKeep: () => void;
  onChange: (value: string) => void;
  onTrash: (trigger: HTMLButtonElement) => void;
}

export default function FolderConflictCandidateCard({
  candidate,
  detail,
  isKeep,
  value,
  error,
  disabled = false,
  inputRef,
  onKeep,
  onChange,
  onTrash,
}: Props) {
  const { t } = useTranslation('folder_grid');
  return (
    <article
      className={`rounded-lg border p-3.5 transition-colors ${
        isKeep 
          ? 'border-success/40 bg-success/5' 
          : 'border-base-content/10 bg-base-200/50'
      }`}
    >
      <div className="flex items-start gap-3">
        <label className="cursor-pointer mt-0.5" aria-label={t('conflict_manager.keep_instead')}>
          <input
            type="radio"
            className={`radio radio-sm ${isKeep ? 'radio-success' : 'radio-neutral/40'}`}
            checked={isKeep}
            onChange={onKeep}
            disabled={disabled}
          />
        </label>
        
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 mb-0.5">
            <h3 className="font-semibold text-sm truncate" title={candidate.folder_name}>
              {candidate.folder_name}
            </h3>
            <span className={`badge badge-xs ${candidate.is_enabled ? 'badge-success' : 'badge-neutral'}`}>
              {candidate.is_enabled ? t('conflict_manager.enabled') : t('conflict_manager.disabled')}
            </span>
            {isKeep && (
              <span className="badge badge-xs badge-success ml-auto">{t('conflict_manager.keep_badge')}</span>
            )}
          </div>
          
          <div className="text-xs text-base-content/50">
            {detail ? (
              <>
                {formatBytes(detail.total_size)} · {t('conflict_manager.file_count', { count: detail.file_count })}
                {detail.partial && (
                  <span className="text-warning ml-1">({t('conflict_manager.partial_details')})</span>
                )}
              </>
            ) : (
              t('conflict_manager.details_unavailable')
            )}
          </div>
          
          <p className="mt-1 text-[11px] text-base-content/40 break-all" title={candidate.path}>
            {candidate.path}
          </p>
        </div>
      </div>

      {!isKeep && (
        <div className="mt-4 ml-8 pl-4 border-l-2 border-base-content/10 flex flex-col gap-3 pb-1">
          <label className="form-control max-w-md">
            <div className="label pt-0 pb-1">
              <span className="label-text flex items-center gap-1.5 text-xs font-medium text-warning">
                <Pencil size={13} aria-hidden />
                {t('conflict_manager.rename_folder')}
              </span>
            </div>
            <input
              ref={inputRef}
              className={`input input-sm input-bordered focus-visible:outline-warning ${error ? 'input-error' : ''}`}
              value={value}
              onChange={(e) => onChange(e.target.value)}
              disabled={disabled}
              aria-invalid={Boolean(error)}
            />
            {error && <span className="mt-1 text-xs text-error">{error}</span>}
          </label>
          
          <div className="flex items-center gap-2.5">
            <span className="text-xs font-medium text-base-content/40">ATAU</span>
            <button
              type="button"
              className="btn btn-xs btn-outline btn-error"
              onClick={(e) => onTrash(e.currentTarget)}
              disabled={disabled}
            >
              <Trash2 size={13} />
              Move to Trash
            </button>
          </div>
        </div>
      )}
    </article>
  );
}
