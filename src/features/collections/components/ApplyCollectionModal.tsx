import { useMemo } from 'react';
import { createPortal } from 'react-dom';
import { AlertTriangle, CheckCircle2, Loader2, Package } from 'lucide-react';
import { useAppStore } from '../../../stores/useAppStore';
import { toast } from '../../../stores/useToastStore';
import {
  useApplyCollection,
  useApplyProgress,
  useCollectionRuntimePreview,
  useCorridorRuntimeSnapshot,
} from '../hooks/useCollections';
import {
  ALL_DISABLED_LABEL,
  getCorridorModeLabel,
  getCorridorStateName,
} from '../../../lib/corridorLabels';

import { buildGroupedModsWithObjectStates } from '../utils/groupMods';
import { ModGroupList } from './ModGroupList';
import { useState } from 'react';
import type { ApplyCollectionResult } from '../../../types/collection';

interface ApplyCollectionModalProps {
  collectionId: string;
  collectionName: string;
  onClose: () => void;
}

export default function ApplyCollectionModal({
  collectionId,
  collectionName,
  onClose,
}: ApplyCollectionModalProps) {
  const { activeGameId, safeMode, setWorkspaceSelectionForCorridor } = useAppStore();
  const applyMutation = useApplyCollection();
  const [applyView, setApplyView] = useState<'preview' | 'progress' | 'success' | 'error'>(
    'preview',
  );
  const [applyResult, setApplyResult] = useState<ApplyCollectionResult | null>(null);
  const [applyError, setApplyError] = useState<string | null>(null);

  const activeStateQuery = useCorridorRuntimeSnapshot(activeGameId, safeMode);
  const targetPreviewQuery = useCollectionRuntimePreview(collectionId, activeGameId);
  const applyProgressQuery = useApplyProgress(activeGameId, applyView === 'progress');

  const isPreviewLoading = activeStateQuery.isLoading || targetPreviewQuery.isLoading;
  const currentObjectStates = activeStateQuery.data?.object_states ?? [];
  const targetObjectStates = targetPreviewQuery.data?.object_states ?? [];

  const relevantObjectIds = useMemo(() => {
    const ids = new Set<string>();

    (activeStateQuery.data?.roots ?? []).forEach((mod) => {
      if (mod.object_id) {
        ids.add(mod.object_id);
      }
    });

    (targetPreviewQuery.data?.roots ?? []).forEach((mod) => {
      if (mod.object_id) {
        ids.add(mod.object_id);
      }
    });

    targetObjectStates.forEach((state) => {
      ids.add(state.object_id);
    });

    const currentStateMap = new Map(
      currentObjectStates.map((state) => [state.object_id, state.is_enabled]),
    );
    const targetStateMap = new Map(targetObjectStates.map((state) => [state.object_id, state.is_enabled]));

    currentObjectStates.forEach((state) => {
      if ((targetStateMap.get(state.object_id) ?? true) !== state.is_enabled) {
        ids.add(state.object_id);
      }
    });

    targetObjectStates.forEach((state) => {
      if ((currentStateMap.get(state.object_id) ?? true) !== state.is_enabled) {
        ids.add(state.object_id);
      }
    });

    return ids;
  }, [
    activeStateQuery.data?.roots,
    currentObjectStates,
    targetPreviewQuery.data?.roots,
    targetObjectStates,
  ]);

  const currentGroups = useMemo(() => {
    return buildGroupedModsWithObjectStates(
      activeStateQuery.data?.roots ?? [],
      currentObjectStates,
      {
        mode: 'preview',
        relevantObjectIds,
      },
    );
  }, [activeStateQuery.data?.roots, currentObjectStates, relevantObjectIds]);

  const targetGroups = useMemo(() => {
    return buildGroupedModsWithObjectStates(targetPreviewQuery.data?.roots ?? [], targetObjectStates, {
      mode: 'preview',
      relevantObjectIds,
    });
  }, [targetPreviewQuery.data?.roots, targetObjectStates, relevantObjectIds]);
  const currentStateName = getCorridorStateName(activeStateQuery.data?.state_name);
  const hasSameCollectionTarget = activeStateQuery.data?.active_collection_id === collectionId;
  const hasObjectOnlyPreview = useMemo(
    () =>
      [...currentGroups, ...targetGroups].some(
        (group) => typeof group.is_enabled === 'boolean' && group.mods.length === 0,
      ),
    [currentGroups, targetGroups],
  );

  const confirmApplyAction = async () => {
    if (!activeGameId) {
      return;
    }
    setApplyError(null);
    setApplyResult(null);
    setApplyView('progress');
    try {
      const result = await applyMutation.mutateAsync({
        collectionId,
        gameId: activeGameId,
        safeMode,
        targetPreview: targetPreviewQuery.data,
      });
      if (result.warnings.length > 0) {
        toast.warning(result.warnings.join(', '));
      }
      setWorkspaceSelectionForCorridor(activeGameId, safeMode, {
        kind: 'stored_collection',
        collection_id: collectionId,
      });
      setApplyResult(result);
      setApplyView('success');
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setApplyError(message);
      setApplyView('error');
    }
  };

  const progress = applyProgressQuery.data;
  const progressLabel = progress?.current_item ?? 'Preparing collection apply...';
  const successSummary = applyResult
    ? `Applied ${collectionName} in ${getCorridorModeLabel(safeMode)} (${applyResult.changed_count} changes).`
    : null;

  return createPortal(
    <dialog className="modal modal-open z-100">
      <div className="modal-box bg-base-200 border border-white/10 shadow-2xl max-w-2xl flex flex-col max-h-[85vh] p-0">
        <div className="p-6 pb-4 border-b border-white/5 shrink-0 bg-base-300/30">
          <h3 className="font-bold text-lg flex items-center gap-2">
            <AlertTriangle size={20} className="text-warning" />
            Apply Collection: <span className="text-white">{collectionName}</span>
          </h3>
          <p className="text-sm text-base-content/70 mt-2">
            Review the active collection before and after apply.
          </p>
        </div>

        <div className="flex-1 overflow-y-auto custom-scrollbar bg-base-100/50 p-6 min-h-75">
          {applyView === 'progress' ? (
            <div className="flex flex-col h-full items-center justify-center p-8 text-center">
              <Loader2 size={32} className="animate-spin mb-4 opacity-70 text-primary" />
              <p className="text-base-content/80 font-medium">
                {progress?.phase === 'updating_db' ? 'Updating database…' : 'Applying collection…'}
              </p>
              <p className="text-sm text-base-content/55 mt-2">{progressLabel}</p>
              <p className="text-xs text-base-content/45 mt-3">
                {progress && progress.total > 0
                  ? `${progress.completed}/${progress.total} steps completed`
                  : 'Preparing rename plan…'}
              </p>
            </div>
          ) : applyView === 'success' ? (
            <div className="flex flex-col h-full items-center justify-center p-8 text-center">
              <CheckCircle2 size={40} className="text-success mb-4" />
              <p className="text-base-content/90 font-semibold">Apply complete</p>
              <p className="text-sm text-base-content/60 mt-2">{successSummary}</p>
              {applyResult && applyResult.warnings.length > 0 && (
                <p className="text-xs text-warning mt-3">{applyResult.warnings.join(', ')}</p>
              )}
            </div>
          ) : applyView === 'error' ? (
            <div className="flex flex-col h-full items-center justify-center p-8 text-center">
              <AlertTriangle size={36} className="text-error mb-4" />
              <p className="text-base-content/90 font-semibold">Apply failed</p>
              <p className="text-sm text-base-content/60 mt-2">
                {progress?.error ?? applyError ?? 'Collection apply failed.'}
              </p>
            </div>
          ) : isPreviewLoading ? (
            <div className="flex flex-col h-full items-center justify-center p-8 text-base-content/50">
              <Loader2 size={32} className="animate-spin mb-4 opacity-50 text-primary" />
              <p>Loading active collection preview...</p>
            </div>
          ) : (
            <div className="space-y-6">
              <div className="grid gap-4 md:grid-cols-2">
                <section className="rounded-xl border border-white/5 bg-base-200/40 overflow-hidden">
                  <div className="border-b border-white/5 bg-base-300/40 px-4 py-3">
                    <p className="text-[11px] uppercase tracking-[0.2em] text-base-content/45">
                      Current
                    </p>
                    <p className="text-lg font-semibold text-base-content/90 mt-1">
                      {currentStateName}
                    </p>
                  </div>
                  <div className="p-4">
                    {currentGroups.length > 0 ? (
                      <>
                        <ModGroupList
                          groups={currentGroups}
                          colorClass="text-base-content/70"
                          emptyGroupMessage="No mods in this object."
                          emptyStateMessage="No active main mods."
                          resetKey={`apply-current-${collectionId}`}
                        />
                      </>
                    ) : (
                      <div className="text-sm text-base-content/40 italic">
                        No active main mods.
                      </div>
                    )}
                  </div>
                </section>

                <section className="rounded-xl border border-white/5 bg-base-200/40 overflow-hidden">
                  <div className="border-b border-white/5 bg-base-300/40 px-4 py-3">
                    <p className="text-[11px] uppercase tracking-[0.2em] text-base-content/45">
                      After Apply
                    </p>
                    <p className="text-lg font-semibold text-base-content/90 mt-1">
                      {collectionName}
                    </p>
                  </div>
                  <div className="p-4">
                    {targetGroups.length > 0 ? (
                      <>
                        <ModGroupList
                          groups={targetGroups}
                          colorClass="text-primary/80"
                          emptyGroupMessage="No mods in this object."
                          emptyStateMessage={`Collection is empty (${ALL_DISABLED_LABEL}).`}
                          resetKey={`apply-target-${collectionId}`}
                        />
                      </>
                    ) : (
                      <div className="text-sm text-base-content/40 italic">
                        Collection is empty ({ALL_DISABLED_LABEL}).
                      </div>
                    )}
                  </div>
                </section>
              </div>

              {hasSameCollectionTarget && (
                <div className="flex flex-col items-center justify-center py-12 text-base-content/50 text-sm">
                  <Package size={48} className="mb-4 opacity-20" />
                  This collection is already the active named state.
                </div>
              )}

              {!hasSameCollectionTarget && hasObjectOnlyPreview && (
                  <div className="text-xs text-base-content/45 italic">
                    This preview also includes object visibility states even when no mods are stored
                    under that object.
                  </div>
                )}
            </div>
          )}
        </div>

        <div className="p-4 border-t border-white/5 bg-base-300/30 shrink-0 flex justify-end gap-2">
          <button
            className="btn btn-ghost btn-sm"
            onClick={onClose}
            disabled={applyView === 'progress'}
          >
            {applyView === 'success' ? 'Close' : 'Cancel'}
          </button>
          <button
            data-testid="modal-apply-btn"
            className="btn btn-primary btn-sm min-w-20"
            onClick={confirmApplyAction}
            disabled={
              applyView === 'progress' ||
              applyView === 'success' ||
              hasSameCollectionTarget ||
              isPreviewLoading
            }
          >
            {applyView === 'progress' ? (
              <Loader2 size={14} className="animate-spin" />
            ) : applyView === 'success' ? (
              'Applied'
            ) : applyView === 'error' ? (
              'Retry Apply'
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
