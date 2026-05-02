/**
 * TrashManagerModal — Lists trashed mods and allows restoring them.
 * Uses native <dialog> with DaisyUI modal styling (consistent with ConfirmDialog).
 */

import { useRef, useEffect, useMemo } from 'react';
import { Trash2, RotateCcw, Loader2, FolderOpen, Clock, HardDrive, XCircle } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useRestoreMod } from '../../hooks/useFolderCoreMutations';
import { useListTrash, useEmptyTrash } from '../../hooks/useFolderMutations';
import { useActiveGame } from '../../hooks/useActiveGame';
import { toast } from '../../stores/useToastStore';
import { formatBytes } from '../../utils/formatters';

interface TrashManagerModalProps {
  open: boolean;
  onClose: () => void;
}

/** Format ISO date to relative or short date string. */
function formatDate(
  isoDate: string,
  t: (key: string, options?: Record<string, unknown>) => string,
): string {
  const date = new Date(isoDate);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 60_000);
  const diffHours = Math.floor(diffMs / 3_600_000);
  const diffDays = Math.floor(diffMs / 86_400_000);

  if (diffMins < 1) return t('common:date.just_now');
  if (diffMins < 60) return t('common:date.mins_ago', { count: diffMins });
  if (diffHours < 24) return t('common:date.hours_ago', { count: diffHours });
  if (diffDays < 7) return t('common:date.days_ago', { count: diffDays });
  return date.toLocaleDateString();
}

export default function TrashManagerModal({ open, onClose }: TrashManagerModalProps) {
  const { t } = useTranslation(['objects', 'common']);
  const dialogRef = useRef<HTMLDialogElement>(null);
  const { data: trashItems = [], isLoading, isError, refetch } = useListTrash(open);
  const { activeGame } = useActiveGame();
  const restoreMutation = useRestoreMod();
  const emptyTrashMutation = useEmptyTrash();

  const handleEmptyTrash = async () => {
    try {
      const count = await emptyTrashMutation.mutateAsync();
      toast.success(t('objects:trash.toasts.empty_success', { count }));
    } catch (err) {
      toast.error(t('objects:trash.toasts.empty_failed', { error: String(err) }));
    }
  };

  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;
    if (open && !dialog.open) dialog.showModal();
    else if (!open && dialog.open) dialog.close();
  }, [open]);

  // Sort by newest first
  const sortedItems = useMemo(
    () =>
      [...trashItems].sort(
        (a, b) => new Date(b.deleted_at).getTime() - new Date(a.deleted_at).getTime(),
      ),
    [trashItems],
  );

  const handleRestore = async (trashId: string, name: string) => {
    try {
      await restoreMutation.mutateAsync({ trashId, gameId: activeGame?.id });
      toast.success(t('objects:trash.toasts.restore_success', { name }));
    } catch (err) {
      toast.error(t('objects:trash.toasts.restore_failed', { error: String(err) }));
    }
  };

  const totalSize = useMemo(
    () => trashItems.reduce((sum, item) => sum + item.size_bytes, 0),
    [trashItems],
  );

  return (
    <dialog ref={dialogRef} className="modal modal-bottom sm:modal-middle" onClose={onClose}>
      <div className="modal-box bg-base-100 border border-base-content/10 shadow-2xl max-w-lg">
        {/* Header */}
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-3">
            <div className="p-2 rounded-lg bg-warning/10 text-warning">
              <Trash2 size={20} />
            </div>
            <div>
              <h3 className="font-semibold text-base text-base-content">
                {t('objects:trash.title')}
              </h3>
              <p className="text-xs text-base-content/50">
                {t('objects:trash.item_count', { count: trashItems.length })}
                {trashItems.length > 0 && ` · ${formatBytes(totalSize)}`}
              </p>
            </div>
          </div>
          <button className="btn btn-sm btn-ghost btn-circle" onClick={onClose}>
            ✕
          </button>
        </div>

        {/* Content */}
        <div className="max-h-80 overflow-y-auto -mx-2 px-2">
          {isLoading && (
            <div className="flex items-center justify-center py-8">
              <Loader2 size={24} className="animate-spin text-primary/50" />
            </div>
          )}

          {isError && (
            <div className="flex flex-col items-center justify-center py-8 gap-2">
              <p className="text-sm text-error/70">{t('objects:trash.error_load')}</p>
              <button className="btn btn-xs btn-ghost text-primary" onClick={() => refetch()}>
                {t('common:actions.retry')}
              </button>
            </div>
          )}

          {!isLoading && !isError && sortedItems.length === 0 && (
            <div className="flex flex-col items-center justify-center py-8 gap-2">
              <FolderOpen size={32} className="text-base-content/15" />
              <p className="text-sm text-base-content/40">{t('objects:trash.empty_state')}</p>
            </div>
          )}

          {!isLoading &&
            !isError &&
            sortedItems.map((item) => (
              <div
                key={item.id}
                className="flex items-center gap-3 py-2.5 px-2 rounded-lg hover:bg-base-200/50 transition-colors group"
              >
                {/* Info */}
                <div className="flex-1 min-w-0">
                  <p className="text-sm font-medium text-base-content truncate">
                    {item.original_name}
                  </p>
                  <div className="flex items-center gap-3 mt-0.5">
                    <span className="flex items-center gap-1 text-xs text-base-content/40">
                      <Clock size={10} />
                      {formatDate(item.deleted_at, t)}
                    </span>
                    <span className="flex items-center gap-1 text-xs text-base-content/40">
                      <HardDrive size={10} />
                      {formatBytes(item.size_bytes)}
                    </span>
                  </div>
                </div>

                {/* Restore button */}
                <button
                  className="btn btn-xs btn-ghost text-primary opacity-0 group-hover:opacity-100 transition-opacity gap-1"
                  onClick={() => handleRestore(item.id, item.original_name)}
                  disabled={restoreMutation.isPending}
                  title={t('objects:trash.restore_tip')}
                >
                  <RotateCcw size={12} />
                  {t('objects:trash.action_restore')}
                </button>
              </div>
            ))}
        </div>

        {/* Footer */}
        <div className="modal-action mt-4">
          {trashItems.length > 0 && (
            <button
              className="btn btn-sm btn-error btn-outline gap-1"
              onClick={handleEmptyTrash}
              disabled={emptyTrashMutation.isPending}
              title={t('objects:trash.empty_tip')}
            >
              {emptyTrashMutation.isPending ? (
                <Loader2 size={12} className="animate-spin" />
              ) : (
                <XCircle size={12} />
              )}
              {t('objects:trash.action_empty')}
            </button>
          )}
          <button className="btn btn-sm btn-ghost" onClick={onClose}>
            {t('common:action.close')}
          </button>
        </div>
      </div>

      {/* Backdrop click closes */}
      <form method="dialog" className="modal-backdrop">
        <button onClick={onClose}>{t('common:action.close')}</button>
      </form>
    </dialog>
  );
}
