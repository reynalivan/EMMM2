import { useEffect, useId, useRef, type KeyboardEvent } from 'react';
import { useTranslation } from 'react-i18next';
import type {
  FolderConflictSummary,
  FolderNameConflictCandidate,
} from '../../../core/tauri/bindings';
import { formatBytes } from '../../../shared/utils/formatters';

interface Props {
  candidate: FolderNameConflictCandidate;
  detail?: FolderConflictSummary;
  returnFocus: HTMLElement;
  submitting: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}

export default function FolderConflictTrashDialog({
  candidate,
  detail,
  returnFocus,
  submitting,
  onCancel,
  onConfirm,
}: Props) {
  const { t } = useTranslation('folder_grid');
  const titleId = useId();
  const dialogRef = useRef<HTMLDivElement>(null);
  const cancelRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    cancelRef.current?.focus();
    return () => returnFocus.focus();
  }, [returnFocus]);

  const handleKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      onCancel();
      return;
    }
    if (event.key !== 'Tab') return;
    const buttons = Array.from(
      dialogRef.current?.querySelectorAll<HTMLButtonElement>('button:not(:disabled)') ?? [],
    );
    const first = buttons[0];
    const last = buttons[buttons.length - 1];
    if (!first || !last) return;
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  };

  return (
    <div
      ref={dialogRef}
      className="absolute inset-0 z-20 grid place-items-center bg-overlay-mask p-4"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby={titleId}
      onKeyDown={handleKeyDown}
    >
      <div className="w-full max-w-sm rounded-xl border border-base-content/10 bg-base-100 p-5 shadow-2xl">
        <h3 id={titleId} className="font-semibold">
          {t('conflict_manager.trash_title')}
        </h3>
        <p className="mt-2 text-sm text-base-content/70">
          {t('conflict_manager.trash_confirm', {
            name: candidate.folder_name,
          })}
        </p>
        <p className="mt-2 text-sm font-medium text-base-content">
          {detail
            ? t('conflict_manager.trash_metadata', {
                size: formatBytes(detail.total_size),
                count: detail.file_count,
              })
            : t('conflict_manager.details_unavailable')}
        </p>
        {detail?.partial && (
          <p className="mt-1 text-xs text-warning" title={detail.warnings.join('\n')}>
            {t('conflict_manager.partial_details')}
          </p>
        )}
        <p className="mt-2 break-all rounded-lg bg-base-200 p-2 text-xs text-base-content/60">
          {candidate.path}
        </p>
        <div className="mt-4 flex justify-end gap-2">
          <button
            ref={cancelRef}
            className="btn btn-sm btn-ghost"
            disabled={submitting}
            onClick={onCancel}
          >
            {t('conflict_manager.cancel')}
          </button>
          <button className="btn btn-sm btn-error" disabled={submitting} onClick={onConfirm}>
            {t('conflict_manager.trash_continue', { name: candidate.folder_name })}
          </button>
        </div>
      </div>
    </div>
  );
}
