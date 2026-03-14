import { useMemo } from 'react';
import { createPortal } from 'react-dom';
import { AlertTriangle, Loader2, Package } from 'lucide-react';
import { useAppStore } from '../../../stores/useAppStore';
import { toast } from '../../../stores/useToastStore';
import {
  useApplyCollection,
  useActiveModsPreview,
  useCollectionPreview,
} from '../hooks/useCollections';

import { groupMods } from '../utils/groupMods';
import { ModGroupList } from './ModGroupList';

interface ApplyCollectionModalProps {
  collectionId: string;
  collectionName: string;
  memberCount: number;
  onClose: () => void;
}

export default function ApplyCollectionModal({
  collectionId,
  collectionName,
  onClose,
}: ApplyCollectionModalProps) {
  const { activeGameId, safeMode } = useAppStore();
  const applyMutation = useApplyCollection();

  const activeModsQuery = useActiveModsPreview(activeGameId, safeMode);
  const targetModsQuery = useCollectionPreview(collectionId, activeGameId);

  const isLoading = activeModsQuery.isLoading || targetModsQuery.isLoading;

  const comparison = useMemo(() => {
    if (!activeModsQuery.data || !targetModsQuery.data) return null;

    const activeIds = new Set(activeModsQuery.data.map((m) => m.id));
    const targetIds = new Set(targetModsQuery.data.map((m) => m.id));

    const toDisable = activeModsQuery.data.filter((m) => !targetIds.has(m.id));
    const toEnable = targetModsQuery.data.filter((m) => !activeIds.has(m.id));
    const unchanged = targetModsQuery.data.filter((m) => activeIds.has(m.id));

    return {
      toDisable: groupMods(toDisable),
      toEnable: groupMods(toEnable),
      unchanged,
    };
  }, [activeModsQuery.data, targetModsQuery.data]);

  const confirmApplyAction = async () => {
    if (!activeGameId) {
      return;
    }
    const result = await applyMutation.mutateAsync({
      collectionId,
      gameId: activeGameId,
    });
    if (result.warnings.length > 0) {
      toast.warning(result.warnings.join(', '));
    }
    useAppStore.getState().setActiveCollectionId(collectionId);
    onClose();
  };

  return createPortal(
    <dialog className="modal modal-open z-100">
      <div className="modal-box bg-base-200 border border-white/10 shadow-2xl max-w-2xl flex flex-col max-h-[85vh] p-0">
        <div className="p-6 pb-4 border-b border-white/5 shrink-0 bg-base-300/30">
          <h3 className="font-bold text-lg flex items-center gap-2">
            <AlertTriangle size={20} className="text-warning" />
            Apply Collection: <span className="text-white">{collectionName}</span>
          </h3>
          <p className="text-sm text-base-content/70 mt-2">
            You are about to switch your active loadout. Review the changes below.
          </p>
        </div>

        <div className="flex-1 overflow-y-auto custom-scrollbar bg-base-100/50 p-6 min-h-75">
          {isLoading ? (
            <div className="flex flex-col h-full items-center justify-center p-8 text-base-content/50">
              <Loader2 size={32} className="animate-spin mb-4 opacity-50 text-primary" />
              <p>Calculating differences...</p>
            </div>
          ) : comparison ? (
            <div className="space-y-6">
              {comparison.toEnable.length > 0 && (
                <div>
                  <h4 className="text-xs font-semibold text-success mb-3 uppercase tracking-wider flex items-center gap-2">
                    <span className="w-2 h-2 rounded-full bg-success inline-block"></span>
                    To Enable
                  </h4>
                  <ModGroupList groups={comparison.toEnable} colorClass="text-success/80" />
                </div>
              )}

              {comparison.toDisable.length > 0 && (
                <div>
                  <h4 className="text-xs font-semibold text-error mb-3 uppercase tracking-wider flex items-center gap-2">
                    <span className="w-2 h-2 rounded-full bg-error inline-block"></span>
                    To Disable
                  </h4>
                  <ModGroupList groups={comparison.toDisable} colorClass="text-error/80" />
                </div>
              )}

              {comparison.unchanged.length > 0 && (
                <div>
                  <h4 className="text-xs font-semibold text-base-content/50 mb-2 uppercase tracking-wider">
                    Unchanged
                  </h4>
                  <div className="text-xs text-base-content/40 italic">
                    {comparison.unchanged.length} mod(s) will remain enabled.
                  </div>
                </div>
              )}

              {comparison.toEnable.length === 0 && comparison.toDisable.length === 0 && (
                <div className="flex flex-col items-center justify-center py-12 text-base-content/50 text-sm">
                  <Package size={48} className="mb-4 opacity-20" />
                  Your current active mods perfectly match this collection. No changes needed.
                </div>
              )}
            </div>
          ) : null}
        </div>

        <div className="p-4 border-t border-white/5 bg-base-300/30 shrink-0 flex justify-end gap-2">
          <button
            className="btn btn-ghost btn-sm"
            onClick={onClose}
            disabled={applyMutation.isPending}
          >
            Cancel
          </button>
          <button
            data-testid="modal-apply-btn"
            className="btn btn-primary btn-sm min-w-20"
            onClick={confirmApplyAction}
            disabled={applyMutation.isPending || isLoading}
          >
            {applyMutation.isPending ? (
              <Loader2 size={14} className="animate-spin" />
            ) : (
              'Confirm Apply'
            )}
          </button>
        </div>
      </div>
      <form method="dialog" className="modal-backdrop">
        <button onClick={onClose}>close</button>
      </form>
    </dialog>,
    document.body,
  );
}
