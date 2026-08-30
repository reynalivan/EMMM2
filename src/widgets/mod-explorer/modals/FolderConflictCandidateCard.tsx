import { Pencil, Trash2, Image as ImageIcon, Info, FolderOpen, File } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { convertFileSrc } from '@tauri-apps/api/core';
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
          ? 'border-success/40 bg-success/5 border-2'
          : 'border-base-content/10 bg-base-200/30 hover:bg-base-200/60'
      }`}
    >
      <div
        className={`flex items-start gap-3 ${!isKeep ? 'cursor-pointer' : ''}`}
        onClick={!isKeep ? onKeep : undefined}
      >
        <div className="mt-0.5 shrink-0">
          <input
            type="radio"
            className={`radio radio-sm ${isKeep ? 'radio-success' : 'radio-neutral/40'}`}
            checked={isKeep}
            readOnly
            disabled={disabled}
            aria-label={t('conflict_manager.keep_instead')}
          />
        </div>

        <div className="shrink-0 w-12 h-12 rounded-lg bg-base-300/50 border border-base-content/10 overflow-hidden flex items-center justify-center shadow-sm">
          {detail?.thumbnail_path ? (
            <img
              src={convertFileSrc(detail.thumbnail_path)}
              alt=""
              className="w-full h-full object-cover"
            />
          ) : (
            <ImageIcon size={20} className="text-base-content/20" />
          )}
        </div>

        <div className="flex-1 min-w-0 flex flex-col justify-center min-h-[3rem]">
          <div className="flex items-center gap-2 mb-1">
            <h3 className="font-semibold text-sm truncate" title={candidate.folder_name}>
              {candidate.folder_name}
            </h3>
            <span
              className={`badge badge-xs ${candidate.is_enabled ? 'badge-success' : 'badge-neutral'}`}
            >
              {candidate.is_enabled
                ? t('conflict_manager.enabled')
                : t('conflict_manager.disabled')}
            </span>
            {isKeep && (
              <span className="badge badge-xs badge-success ml-auto shadow-sm">
                {t('conflict_manager.keep_badge')}
              </span>
            )}
          </div>

          <div className="dropdown dropdown-hover dropdown-bottom z-20">
            <div
              tabIndex={0}
              role="button"
              className="inline-flex items-center gap-1.5 text-[11px] font-medium text-base-content/50 hover:text-base-content transition-colors bg-base-200/50 hover:bg-base-200 px-2 py-0.5 rounded-md"
            >
              <Info size={12} />
              {detail ? (
                <>
                  {formatBytes(detail.total_size)} ·{' '}
                  {t('conflict_manager.file_count', { count: detail.file_count })}
                  {detail.partial && (
                    <span className="text-warning ml-0.5">
                      ({t('conflict_manager.partial_details')})
                    </span>
                  )}
                </>
              ) : (
                t('conflict_manager.details_unavailable')
              )}
            </div>

            {detail && (
              <div
                tabIndex={0}
                className="dropdown-content mt-2 w-72 p-0 shadow-xl bg-base-100 rounded-xl border border-base-content/10 overflow-hidden text-xs"
                onClick={(e) => e.stopPropagation()}
              >
                <div className="bg-base-200/50 p-3 border-b border-base-content/5">
                  <h4 className="font-semibold text-base-content mb-2.5 flex items-center gap-2">
                    <FolderOpen size={14} className="text-base-content/70" />
                    {t('conflict_manager.folder_info')}
                  </h4>
                  <div className="grid grid-cols-2 gap-3 text-[11px]">
                    <div>
                      <span className="text-base-content/40 block mb-0.5 uppercase tracking-wider font-semibold text-[9px]">
                        {t('conflict_manager.created')}
                      </span>
                      <span className="font-medium">
                        {detail.created_at
                          ? new Date(detail.created_at).toLocaleDateString(undefined, {
                              year: 'numeric',
                              month: 'short',
                              day: 'numeric',
                            })
                          : '-'}
                      </span>
                    </div>
                    <div>
                      <span className="text-base-content/40 block mb-0.5 uppercase tracking-wider font-semibold text-[9px]">
                        {t('conflict_manager.modified')}
                      </span>
                      <span className="font-medium">
                        {detail.modified_at
                          ? new Date(detail.modified_at).toLocaleDateString(undefined, {
                              year: 'numeric',
                              month: 'short',
                              day: 'numeric',
                            })
                          : '-'}
                      </span>
                    </div>
                  </div>
                </div>

                <div className="p-3">
                  <div className="text-[9px] font-bold uppercase tracking-wider text-base-content/40 mb-2.5 flex justify-between items-center">
                    <span>{t('conflict_manager.contents', { count: detail.file_count })}</span>
                  </div>
                  <div className="space-y-2 max-h-40 overflow-y-auto pr-1">
                    {detail.files.map((f, i) => (
                      <div
                        key={i}
                        className="flex items-start gap-2 text-base-content/80 text-[11px]"
                      >
                        <File size={12} className="shrink-0 mt-0.5 opacity-40" />
                        <span className="truncate leading-tight" title={f}>
                          {f}
                        </span>
                      </div>
                    ))}
                    {detail.file_count > detail.files.length && (
                      <div className="text-base-content/40 italic text-[10px] pl-5 mt-1">
                        {t('conflict_manager.more_files', {
                          count: detail.file_count - detail.files.length,
                        })}
                      </div>
                    )}
                  </div>
                </div>

                <div className="bg-base-200/30 p-3 border-t border-base-content/5">
                  <div className="text-[9px] font-bold uppercase tracking-wider text-base-content/40 mb-1.5">
                    {t('conflict_manager.full_path')}
                  </div>
                  <div className="text-[10px] text-base-content/60 break-all select-text font-mono leading-relaxed">
                    {candidate.path}
                  </div>
                </div>
              </div>
            )}
          </div>
        </div>
      </div>

      {!isKeep && (
        <div className="mt-4 ml-8 pl-4 border-l-2 border-base-content/10 flex flex-col gap-4 pb-1">
          <label className="form-control max-w-md">
            <div className="label pt-0 pb-1.5">
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

          <div>
            <button
              type="button"
              className="btn btn-xs btn-outline btn-error opacity-80 hover:opacity-100"
              onClick={(e) => onTrash(e.currentTarget)}
              disabled={disabled}
            >
              <Trash2 size={13} />
              {t('conflict_manager.move_to_trash')}
            </button>
          </div>
        </div>
      )}
    </article>
  );
}
