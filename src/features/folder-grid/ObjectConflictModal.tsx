import { useState, useRef, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { AlertTriangle, Info, ShieldAlert, Zap, Ghost } from 'lucide-react';
import type { ModFolder } from '../../types/mod';
import type { DuplicateInfo } from '../../types/scanner';
import { useAppStore } from '../../stores/useAppStore';
import { commands } from '../../lib/bindings';
import { toast } from '../../stores/useToastStore';
import { useQueryClient } from '@tanstack/react-query';
import { applyRuntimeMutationResult } from '../workspace-runtime/actions/sharedRuntimeResultMapper';
import { useWorkspaceSwitchActions } from '../workspace-runtime/actions/useWorkspaceSwitchActions';

interface ObjectConflictModalProps {
  open: boolean;
  folder: ModFolder | null;
  duplicates: DuplicateInfo[];
  onClose: () => void;
}

export default function ObjectConflictModal({
  open,
  folder,
  duplicates,
  onClose,
}: ObjectConflictModalProps) {
  const { t } = useTranslation(['folder_grid', 'common']);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [isResolving, setIsResolving] = useState(false);
  const dialogRef = useRef<HTMLDialogElement>(null);
  const queryClient = useQueryClient();
  const activeGameId = useAppStore((state) => state.activeGameId);
  const switchActions = useWorkspaceSwitchActions();

  useEffect(() => {
    if (open) {
      setSelectedId(folder?.path || null);
      if (dialogRef.current && !dialogRef.current.open) {
        dialogRef.current.showModal();
      }
    } else {
      if (dialogRef.current && dialogRef.current.open) {
        dialogRef.current.close();
      }
    }
  }, [open, folder]);

  const handleKeepSelected = async () => {
    if (!selectedId || !activeGameId) return;
    setIsResolving(true);
    try {
      await switchActions.resolveDuplicateEnableOnly({ path: selectedId });
      toast.success(t('folder_grid:conflicts.toast.resolved'));
      onClose();
    } catch (err) {
      toast.error(t('folder_grid:conflicts.toast.resolve_failed', { error: String(err) }));
    } finally {
      setIsResolving(false);
    }
  };

  const handleIgnore = async () => {
    if (!folder || !activeGameId || duplicates.length === 0) return;
    setIsResolving(true);
    try {
      // All duplicates share the same object_id
      const objectId = duplicates[0].object_id;
      const modIds = [folder.id || folder.path, ...duplicates.map((d) => d.mod_id)];
      const sortedModIds = [...modIds].sort();

      await commands.ignoreObjectConflict({
        gameId: activeGameId,
        objectId,
        modIds: sortedModIds,
      });

      await switchActions.setFolderPathEnabled(folder.path, true, {
        syncExplorerPath: false,
      });

      toast.success(t('folder_grid:conflicts.toast.ignored'));
      await applyRuntimeMutationResult(queryClient, 'conflictsOnly');
      onClose();
    } catch (err) {
      toast.error(t('folder_grid:conflicts.toast.ignore_failed', { error: String(err) }));
    } finally {
      setIsResolving(false);
    }
  };

  if (!folder) return null;

  const allMods = [
    {
      id: folder.id || folder.path,
      path: folder.path,
      name: folder.name,
      incoming: true,
    },
    ...duplicates.map((d) => ({
      id: d.mod_id,
      path: d.folder_path,
      name: d.actual_name,
      incoming: false,
    })),
  ];

  return (
    <dialog ref={dialogRef} className="modal modal-bottom sm:modal-middle" onClose={onClose}>
      <div className="modal-box bg-base-100 border border-base-content/10 shadow-2xl max-w-lg p-0 overflow-hidden">
        {/* Banner */}
        <div className="bg-warning/10 px-6 py-4 border-b border-warning/20 flex items-center gap-3">
          <AlertTriangle className="text-warning" size={24} />
          <div>
            <h3 className="font-bold text-lg text-base-content">
              {t('folder_grid:conflicts.title')}
            </h3>
            <p className="text-xs text-base-content/60">{t('folder_grid:conflicts.desc')}</p>
          </div>
        </div>

        <div className="p-6">
          <p className="text-sm mb-4 text-base-content/80 font-medium">
            {t('folder_grid:conflicts.resolution_desc')}
          </p>

          <div className="space-y-2 max-h-60 overflow-y-auto pr-2 scrollbar-thin">
            {allMods.map((m) => (
              <label
                key={m.path}
                className={`flex items-center gap-3 p-3 rounded-xl border transition-all cursor-pointer ${
                  selectedId === m.path
                    ? 'bg-primary/10 border-primary ring-1 ring-primary/20'
                    : 'bg-base-200/50 border-transparent hover:border-base-content/10'
                }`}
              >
                <input
                  type="radio"
                  name="resolution"
                  className="radio radio-primary radio-sm"
                  checked={selectedId === m.path}
                  onChange={() => setSelectedId(m.path)}
                  disabled={isResolving}
                />
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="font-semibold text-sm truncate text-base-content">
                      {m.name}
                    </span>
                    {m.incoming && (
                      <span className="badge badge-primary badge-xs py-2 px-2 font-bold uppercase tracking-tighter">
                        {t('folder_grid:conflicts.incoming')}
                      </span>
                    )}
                  </div>
                  <span className="text-[10px] text-base-content/50 truncate block">{m.path}</span>
                </div>
                {selectedId === m.path ? (
                  <Info size={18} className="text-primary mt-0.5 shrink-0" />
                ) : (
                  <ShieldAlert size={16} className="text-base-content/30" />
                )}
              </label>
            ))}
          </div>

          <div className="mt-8 flex flex-col sm:flex-row gap-2">
            <button
              className="btn btn-ghost btn-sm flex-1 font-bold"
              onClick={onClose}
              disabled={isResolving}
            >
              {t('common:actions.cancel')}
            </button>
            <button
              className={`btn btn-warning btn-outline btn-sm gap-2 flex-1 ${isResolving ? 'loading' : ''}`}
              onClick={handleIgnore}
              disabled={isResolving}
            >
              {!isResolving && <Ghost size={14} />}
              {t('folder_grid:conflicts.ignore_warning')}
            </button>
            <button
              className={`btn btn-primary btn-sm min-w-35 gap-2 flex-1 ${isResolving ? 'loading' : ''}`}
              onClick={handleKeepSelected}
              disabled={!selectedId || isResolving}
            >
              {!isResolving && <Zap size={14} />}
              {t('folder_grid:conflicts.keep_selected')}
            </button>
          </div>
        </div>
      </div>
      <form method="dialog" className="modal-backdrop bg-overlay-mask backdrop-blur-sm">
        <button tabIndex={-1}>{t('common:actions.close')}</button>
      </form>
    </dialog>
  );
}
