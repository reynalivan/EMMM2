import { useMemo, useState } from 'react';
import { createPortal } from 'react-dom';
import {
  AlertTriangle,
  Loader2,
  ChevronDown,
  ChevronRight,
  Package,
  ShieldAlert,
} from 'lucide-react';
import { useAppStore } from '../../../stores/useAppStore';
import { toast } from '../../../stores/useToastStore';
import {
  useApplyCollection,
  useActiveModsPreview,
  useCollectionPreview,
} from '../hooks/useCollections';
import type { CollectionPreviewMod } from '../../../types/collection';
import { ModListRow } from './CollectionWorkspace';

interface ApplyCollectionModalProps {
  collectionId: string;
  collectionName: string;
  memberCount: number;
  onClose: () => void;
}

interface GroupedMod {
  id: string;
  name: string;
  type: string;
  mods: CollectionPreviewMod[];
  unsafeCount: number;
}

function groupMods(mods: CollectionPreviewMod[]): GroupedMod[] {
  const objectsMap = new Map<string, GroupedMod>();
  let hasUncategorized = false;
  const uncategorizedMods: CollectionPreviewMod[] = [];
  let uncategorizedUnsafeCount = 0;

  mods.forEach((mod) => {
    if (mod.object_name) {
      const groupKey = mod.object_id || mod.object_name;
      if (!objectsMap.has(groupKey)) {
        objectsMap.set(groupKey, {
          id: groupKey,
          name: mod.object_name,
          type: mod.object_type || 'Other',
          mods: [],
          unsafeCount: 0,
        });
      }
      const obj = objectsMap.get(groupKey)!;
      obj.mods.push(mod);
      if (!mod.is_safe) obj.unsafeCount += 1;
    } else {
      hasUncategorized = true;
      uncategorizedMods.push(mod);
      if (!mod.is_safe) uncategorizedUnsafeCount += 1;
    }
  });

  const groupedObjects = Array.from(objectsMap.values());
  if (hasUncategorized) {
    groupedObjects.push({
      id: 'uncategorized',
      name: 'Uncategorized',
      type: 'Other',
      mods: uncategorizedMods,
      unsafeCount: uncategorizedUnsafeCount,
    });
  }

  const typeOrder = ['Character', 'Weapon', 'UI', 'Other'];
  groupedObjects.sort((a, b) => {
    const idxA = typeOrder.indexOf(a.type);
    const idxB = typeOrder.indexOf(b.type);
    if (idxA !== -1 && idxB !== -1 && idxA !== idxB) return idxA - idxB;
    if (idxA !== -1 && idxB === -1) return -1;
    if (idxA === -1 && idxB !== -1) return 1;
    return a.name.localeCompare(b.name);
  });

  return groupedObjects;
}

function ModGroupList({ groups, colorClass }: { groups: GroupedMod[]; colorClass: string }) {
  const [expanded, setExpanded] = useState<Set<string>>(new Set(groups.map((g) => g.id)));

  const toggle = (id: string) => {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  return (
    <div className="space-y-2">
      {groups.map((obj) => {
        const isExpanded = expanded.has(obj.id);
        return (
          <div
            key={obj.id}
            className="border border-white/5 rounded-lg overflow-hidden bg-base-100/30"
          >
            <button
              onClick={() => toggle(obj.id)}
              className="w-full flex items-center justify-between px-3 py-2 hover:bg-base-300/30 transition-colors"
            >
              <div className="flex items-center gap-2">
                <div className="text-base-content/50">
                  {isExpanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
                </div>
                <span className="font-semibold text-xs">{obj.name}</span>
                <span className="text-[9px] text-base-content/40 uppercase tracking-widest">
                  {obj.type}
                </span>
              </div>
              <div className="flex items-center gap-2">
                <span className={`text-[10px] ${colorClass}`}>{obj.mods.length} mods</span>
                {obj.unsafeCount > 0 && (
                  <span className="flex items-center gap-0.5 text-[10px] text-error/70">
                    <ShieldAlert size={10} />
                    {obj.unsafeCount}
                  </span>
                )}
              </div>
            </button>
            {isExpanded && (
              <div className="border-t border-white/5 py-1 px-1">
                {obj.mods.map((mod) => (
                  <ModListRow key={mod.id} mod={mod} />
                ))}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}

export default function ApplyCollectionModal({
  collectionId,
  collectionName,
  onClose,
}: ApplyCollectionModalProps) {
  const { activeGameId } = useAppStore();
  const applyMutation = useApplyCollection();

  const activeModsQuery = useActiveModsPreview(activeGameId);
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
