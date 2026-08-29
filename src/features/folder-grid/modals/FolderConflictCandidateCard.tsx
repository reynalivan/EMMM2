import { ArrowRight, Check, Pencil, Trash2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { FolderConflictSummary, FolderNameConflictCandidate } from '../../../lib/bindings';
import { formatBytes } from '../../../utils/formatters';

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

function targetFolderName(candidate: FolderNameConflictCandidate, baseName: string): string {
  const currentName = candidate.folder_name;
  if (currentName.toLowerCase().endsWith(candidate.base_name.toLowerCase())) {
    return `${currentName.slice(0, currentName.length - candidate.base_name.length)}${baseName}`;
  }
  return baseName;
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
  const finalFolderName = targetFolderName(candidate, value);
  return (
    <article
      className={`rounded-xl border p-3 ${isKeep ? 'border-success/40 bg-success/5' : error ? 'border-error/50 bg-error/5' : 'border-warning/30 bg-warning/5'}`}
    >
      <div className="mb-2 flex items-center justify-between gap-2">
        <div className="flex min-w-0 items-center gap-2">
          <span
            className={`badge badge-sm ${candidate.is_enabled ? 'badge-success' : 'badge-neutral'}`}
          >
            {candidate.is_enabled ? t('conflict_manager.enabled') : t('conflict_manager.disabled')}
          </span>
          <span className={`badge badge-sm ${isKeep ? 'badge-success' : 'badge-warning'}`}>
            {isKeep ? t('conflict_manager.keep_badge') : t('conflict_manager.rename_badge')}
          </span>
        </div>
        <span className="text-xs tabular-nums text-base-content/50">
          {detail
            ? `${formatBytes(detail.total_size)} · ${t('conflict_manager.file_count', { count: detail.file_count })}`
            : t('conflict_manager.details_unavailable')}
        </span>
      </div>
      <h3 className="truncate text-sm font-semibold" title={candidate.folder_name}>
        {candidate.folder_name}
      </h3>
      <p className="mt-2 break-all text-[11px] text-base-content/45" title={candidate.path}>
        {candidate.path}
      </p>
      {detail?.partial && (
        <p className="mt-2 text-xs text-warning" title={detail.warnings.join('\n')}>
          {t('conflict_manager.partial_details')}
        </p>
      )}

      <div className="mt-3 rounded-lg border border-base-content/10 bg-base-100/70 p-2.5">
        <div className="mb-2 flex items-center justify-between gap-2">
          <span className="text-[11px] font-semibold uppercase tracking-wide text-base-content/50">
            {t('conflict_manager.action')}
          </span>
          <button
            type="button"
            className={`btn btn-xs ${isKeep ? 'btn-success' : 'btn-ghost'}`}
            aria-label={t('conflict_manager.keep_action_label', { path: candidate.path })}
            aria-pressed={isKeep}
            disabled={disabled}
            onClick={onKeep}
          >
            <Check size={13} aria-hidden />
            {isKeep ? t('conflict_manager.keep_current_name') : t('conflict_manager.keep_instead')}
          </button>
        </div>

        {isKeep ? (
          <div className="flex items-center gap-2 text-xs text-success">
            <Check size={14} aria-hidden />
            <span>{t('conflict_manager.no_filesystem_changes')}</span>
          </div>
        ) : (
          <>
            <p className="mb-2 flex items-center gap-1.5 text-xs font-medium text-warning">
              <Pencil size={13} aria-hidden />
              {t('conflict_manager.rename_folder')}
            </p>
            <label className="form-control">
              <span className="label-text mb-1 text-xs">{t('conflict_manager.new_base_name')}</span>
              <input
                ref={inputRef}
                className={`input input-sm input-bordered w-full focus-visible:outline focus-visible:outline-2 focus-visible:outline-primary ${error ? 'input-error' : ''}`}
                value={value}
                disabled={disabled}
                onChange={(event) => onChange(event.target.value)}
                aria-invalid={Boolean(error)}
              />
              {error && <span className="mt-1 text-xs text-error">{error}</span>}
            </label>
            <div className="mt-2 flex min-w-0 items-center gap-1.5 text-[11px] text-base-content/60">
              <span className="truncate" title={candidate.folder_name}>
                {candidate.folder_name}
              </span>
              <ArrowRight size={12} className="shrink-0" aria-hidden />
              <span className="truncate font-medium text-base-content" title={finalFolderName}>
                {finalFolderName}
              </span>
            </div>
          </>
        )}
      </div>

      <div className="mt-3">
        <button
          type="button"
          className="btn btn-xs btn-ghost w-full justify-start text-error hover:bg-error/10 focus-visible:outline focus-visible:outline-2 focus-visible:outline-error"
          aria-label={t('conflict_manager.trash_action_label', {
            name: candidate.folder_name,
            path: candidate.path,
          })}
          disabled={disabled}
          onClick={(event) => onTrash(event.currentTarget)}
        >
          <Trash2 size={14} />
          {t('conflict_manager.trash_action', { name: candidate.folder_name })}
        </button>
      </div>
    </article>
  );
}
