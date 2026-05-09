import { useRef, useEffect } from 'react';
import { X, Trash2, ShieldAlert, Ghost, Info } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useAppStore } from '../../stores/useAppStore';
import { commands } from '../../lib/bindings';
import { toast } from '../../stores/useToastStore';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import type { IgnoredConflict } from '../../types/scanner';
import { applyRuntimeMutationResult } from '../workspace-runtime/actions/sharedRuntimeResultMapper';

interface IgnoreManagementModalProps {
  open: boolean;
  onClose: () => void;
}

export default function IgnoreManagementModal({ open, onClose }: IgnoreManagementModalProps) {
  const { t } = useTranslation(['folder_grid', 'common']);
  const dialogRef = useRef<HTMLDialogElement>(null);
  const queryClient = useQueryClient();
  const activeGameId = useAppStore((state) => state.activeGameId);

  const { data: ignoredConflicts = [], refetch } = useQuery<IgnoredConflict[]>({
    queryKey: ['ignored-conflicts', activeGameId],
    queryFn: () => commands.listIgnoredObjectConflicts({ gameId: activeGameId! }),
    enabled: open && !!activeGameId,
  });

  useEffect(() => {
    if (open) {
      if (dialogRef.current && !dialogRef.current.open) {
        dialogRef.current.showModal();
      }
    } else {
      if (dialogRef.current && dialogRef.current.open) {
        dialogRef.current.close();
      }
    }
  }, [open]);

  const handleRevoke = async (objectId: string) => {
    if (!activeGameId) return;
    try {
      await commands.revokeObjectConflict({ gameId: activeGameId, objectId });
      toast.success(t('ignore_management.toast_revoked'));
      refetch();
      void applyRuntimeMutationResult(queryClient, ['workspaceStructure', 'conflictsOnly']);
    } catch (err) {
      toast.error(t('ignore_management.toast_revoke_failed', { error: String(err) }));
    }
  };

  return (
    <dialog ref={dialogRef} className="modal modal-bottom sm:modal-middle" onClose={onClose}>
      <div className="modal-box bg-base-100 border border-base-content/10 shadow-2xl max-w-2xl p-0 overflow-hidden">
        {/* Header */}
        <div className="bg-base-200/50 px-6 py-4 border-b border-base-content/10 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <Ghost className="text-primary" size={24} />
            <div>
              <h3 className="font-bold text-lg text-base-content">
                {t('ignore_management.title')}
              </h3>
              <p className="text-xs text-base-content/60">{t('ignore_management.subtitle')}</p>
            </div>
          </div>
          <button className="btn btn-ghost btn-sm btn-circle" onClick={onClose}>
            <X size={20} />
          </button>
        </div>

        <div className="p-6">
          {ignoredConflicts.length === 0 ? (
            <div className="py-12 flex flex-col items-center justify-center text-center opacity-40">
              <ShieldAlert size={48} className="mb-4" />
              <p className="text-sm font-medium">{t('ignore_management.empty_title')}</p>
              <p className="text-xs">{t('ignore_management.empty_subtitle')}</p>
            </div>
          ) : (
            <div className="space-y-3 max-h-[50vh] overflow-y-auto pr-2 scrollbar-thin">
              {ignoredConflicts.map((item) => (
                <div
                  key={item.id}
                  className="bg-base-200/30 border border-base-content/5 rounded-xl p-4 flex items-center justify-between gap-4 group hover:border-primary/20 hover:bg-primary/5 transition-all"
                >
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 mb-1">
                      <span className="font-bold text-sm text-base-content truncate">
                        {item.object_name || t('ignore_management.unknown_object')}
                      </span>
                      <span className="text-[10px] bg-base-content/10 text-base-content/60 px-1.5 py-0.5 rounded font-mono uppercase">
                        {item.object_id.slice(0, 8)}
                      </span>
                    </div>
                    <div className="flex flex-wrap gap-1.5">
                      {item.mod_names.map((name, idx) => (
                        <span
                          key={idx}
                          className="badge badge-ghost badge-sm border-base-content/10 text-[10px] py-1"
                        >
                          {name}
                        </span>
                      ))}
                    </div>
                  </div>
                  <button
                    className="btn btn-ghost btn-sm text-error opacity-0 group-hover:opacity-100 transition-opacity gap-2"
                    onClick={() => handleRevoke(item.object_id)}
                    title={t('ignore_management.revoke_title')}
                  >
                    <Trash2 size={16} />
                    <span className="hidden sm:inline">{t('ignore_management.revoke')}</span>
                  </button>
                </div>
              ))}
            </div>
          )}

          <div className="mt-8 flex items-start gap-3 bg-primary/5 p-4 rounded-xl border border-primary/10">
            <Info size={18} className="text-primary mt-0.5 shrink-0" />
            <p className="text-[11px] text-base-content/70 leading-relaxed">
              {t('ignore_management.note')}
            </p>
          </div>

          <div className="mt-6 flex justify-end">
            <button className="btn btn-primary btn-sm px-8" onClick={onClose}>
              {t('common:actions.done')}
            </button>
          </div>
        </div>
      </div>
      <form method="dialog" className="modal-backdrop bg-base-300/40 backdrop-blur-sm">
        <button tabIndex={-1}>{t('common:actions.close')}</button>
      </form>
    </dialog>
  );
}
