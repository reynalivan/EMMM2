import { useMemo } from 'react';
import { createPortal } from 'react-dom';
import { invoke } from '@tauri-apps/api/core';
import { ShieldCheck, ShieldAlert, Loader2, ArrowRight } from 'lucide-react';
import { useQuery } from '@tanstack/react-query';
import { useAppStore } from '../../stores/useAppStore';
import { ModGroupList } from '../collections/components/ModGroupList';
import { buildGroupedModsWithObjectStates } from '../collections/utils/groupMods';
import { corridorPreviewKeys } from '../collections/queryKeys';
import { useCorridorRuntimeSnapshot } from '../collections/hooks/useCollections';
import {
  ALL_DISABLED_LABEL,
  buildCorridorModeSwitchTitle,
  buildCorridorEmptyStateLabel,
  buildLeavingCorridorLabel,
  buildMissingTargetCorridorDescription,
  buildTargetCorridorDescription,
  buildTargetCorridorLabel,
  getCorridorStateName,
} from '../../lib/corridorLabels';
import type { CorridorPreview } from '../../types/collection';

function buildRelevantObjectIds(preview: CorridorPreview | undefined): Set<string> {
  const relevantObjectIds = new Set<string>();
  if (!preview) {
    return relevantObjectIds;
  }

  preview.leaving_mods.forEach((mod) => {
    if (mod.object_id) {
      relevantObjectIds.add(mod.object_id);
    }
  });
  preview.target_mods.forEach((mod) => {
    if (mod.object_id) {
      relevantObjectIds.add(mod.object_id);
    }
  });
  preview.leaving_object_states.forEach((state) => {
    relevantObjectIds.add(state.object_id);
  });
  preview.target_object_states.forEach((state) => {
    relevantObjectIds.add(state.object_id);
  });

  return relevantObjectIds;
}

function buildLeavingSubtitle(preview: CorridorPreview | undefined): string {
  return getCorridorStateName(preview?.leaving_state_name);
}

function buildTargetSubtitle(preview: CorridorPreview | undefined): string {
  if (!preview || preview.target_state_kind === 'none') {
    return ALL_DISABLED_LABEL;
  }

  return getCorridorStateName(preview.target_state_name);
}

function buildLeavingDescription(): string {
  return 'Current Active Mods';
}

function buildTargetDescription(preview: CorridorPreview | undefined): string {
  if (!preview || preview.target_state_kind === 'none') {
    return buildTargetCorridorDescription(null);
  }

  return buildTargetCorridorDescription(preview.target_state_name);
}

function buildTargetEmptyState(preview: CorridorPreview | undefined): string {
  if (!preview || preview.target_state_kind === 'none') {
    return buildMissingTargetCorridorDescription();
  }

  return buildCorridorEmptyStateLabel(preview.target_state_name);
}

interface ModeSwitchConfirmModalProps {
  open: boolean;
  targetSafeMode: boolean;
  onClose: () => void;
  onConfirm: () => void;
}

export default function ModeSwitchConfirmModal({
  open,
  targetSafeMode,
  onClose,
  onConfirm,
}: ModeSwitchConfirmModalProps) {
  const { activeGameId, safeMode } = useAppStore();
  const currentRuntimeQuery = useCorridorRuntimeSnapshot(activeGameId, safeMode);
  const currentStateToken = useMemo(() => {
    if (!currentRuntimeQuery.data) {
      return 'unknown';
    }

    return [
      currentRuntimeQuery.data.state_kind,
      currentRuntimeQuery.data.active_collection_id ?? '',
      currentRuntimeQuery.data.state_name ?? '',
      currentRuntimeQuery.data.signature,
    ].join(':');
  }, [currentRuntimeQuery.data]);

  const { data: preview, isLoading } = useQuery<CorridorPreview>({
    queryKey: corridorPreviewKeys.detail(
      activeGameId ?? '',
      safeMode,
      targetSafeMode,
      currentStateToken,
    ),
    queryFn: () => invoke<CorridorPreview>('preview_corridor_switch', { targetEnabled: targetSafeMode }),
    enabled: open && !!activeGameId && currentRuntimeQuery.status === 'success',
    staleTime: 0,
  });
  const isPreviewLoading = currentRuntimeQuery.status !== 'success' || isLoading;

  const leavingGroups = useMemo(() => {
    if (!preview) {
      return [];
    }
    const relevantObjectIds = buildRelevantObjectIds(preview);
    return buildGroupedModsWithObjectStates(preview.leaving_mods, preview.leaving_object_states, {
      mode: 'preview',
      relevantObjectIds,
    });
  }, [preview]);

  const targetGroups = useMemo(() => {
    if (!preview) {
      return [];
    }
    const relevantObjectIds = buildRelevantObjectIds(preview);
    return buildGroupedModsWithObjectStates(preview.target_mods, preview.target_object_states, {
      mode: 'preview',
      relevantObjectIds,
    });
  }, [preview]);

  if (!open) return null;

  return createPortal(
    <dialog className="modal modal-open z-1000">
      <div className="modal-box bg-base-200 border border-white/10 shadow-2xl max-w-5xl flex flex-col max-h-[85vh] p-0 overflow-hidden">
        {/* Header */}
        <div className="p-6 pb-4 border-b border-white/5 shrink-0 bg-base-300/30">
          <h3 className="font-bold text-lg flex items-center gap-2 text-warning mb-1">
            {targetSafeMode ? <ShieldCheck size={20} /> : <ShieldAlert size={20} />}
            {buildCorridorModeSwitchTitle(targetSafeMode)}
          </h3>
          <p className="text-sm text-base-content/70">
            Review the changes to your active loadout before switching corridors.
          </p>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-hidden flex flex-col bg-base-100/50 min-h-0">
          {isPreviewLoading ? (
            <div className="flex-1 flex flex-col items-center justify-center p-8 text-base-content/50 min-h-75">
              <Loader2 size={32} className="animate-spin mb-4 opacity-50 text-warning" />
              <p>Loading preview...</p>
            </div>
          ) : (
            <div className="flex-1 flex flex-col sm:flex-row min-h-0">
              {/* Left Column: Leaving */}
              <div className="flex-1 border-r border-white/5 flex flex-col min-h-0 sm:max-w-[50%]">
                <div
                  className={`p-4 border-b border-white/5 ${targetSafeMode ? 'bg-error/5' : 'bg-success/5'} shrink-0`}
                >
                  <h4
                    className={`text-[11px] uppercase tracking-[0.2em] flex justify-between items-center ${targetSafeMode ? 'text-error/70' : 'text-success/70'} mb-2`}
                  >
                    {buildLeavingCorridorLabel(targetSafeMode)}
                    <span
                      className={`badge badge-sm ${targetSafeMode ? 'badge-error' : 'badge-success'} badge-outline`}
                    >
                      Snapshot
                    </span>
                  </h4>
                  <p
                    className={`text-lg font-semibold break-all leading-tight ${targetSafeMode ? 'text-error/90' : 'text-success/90'}`}
                  >
                    {buildLeavingSubtitle(preview)}
                  </p>
                  <p className="text-xs text-base-content/45 mt-1">{buildLeavingDescription()}</p>
                </div>
                <div className="p-4 overflow-y-auto custom-scrollbar flex-1">
                  <ModGroupList
                    groups={leavingGroups}
                    colorClass={targetSafeMode ? 'text-error' : 'text-success'}
                  />
                </div>
              </div>

              {/* Center Arrow */}
              <div className="hidden sm:flex items-center justify-center w-8 -mx-4 z-10 text-base-content/30 opacity-50 relative pointer-events-none">
                <div className="bg-base-200 rounded-full p-1 border border-white/10">
                  <ArrowRight size={20} />
                </div>
              </div>

              {/* Right Column: Target */}
              <div className="flex-1 flex flex-col min-h-0 sm:max-w-[50%]">
                <div
                  className={`p-4 border-b border-white/5 ${targetSafeMode ? 'bg-success/5' : 'bg-error/5'} shrink-0`}
                >
                  <h4
                    className={`text-[11px] uppercase tracking-[0.2em] flex justify-between items-center ${targetSafeMode ? 'text-success/70' : 'text-error/70'} mb-2`}
                  >
                    {buildTargetCorridorLabel(targetSafeMode)}
                    <span
                      className={`badge badge-sm ${targetSafeMode ? 'badge-success' : 'badge-error'} badge-outline`}
                    >
                      Restore
                    </span>
                  </h4>
                  <p
                    className={`text-lg font-semibold break-all leading-tight ${targetSafeMode ? 'text-success/90' : 'text-error/90'}`}
                  >
                    {buildTargetSubtitle(preview)}
                  </p>
                  <p className="text-xs text-base-content/45 mt-1">
                    {buildTargetDescription(preview)}
                  </p>
                </div>
                <div className="p-4 overflow-y-auto custom-scrollbar flex-1">
                  {targetGroups.length > 0 ? (
                    <ModGroupList
                      groups={targetGroups}
                      colorClass={targetSafeMode ? 'text-success' : 'text-error'}
                    />
                  ) : (
                    <div
                      className={`text-center p-8 text-sm text-base-content/40 border ${targetSafeMode ? 'border-success/20' : 'border-error/20'} border-dashed rounded-lg bg-base-100/10 m-2 mt-4`}
                    >
                      {buildTargetEmptyState(preview)}
                    </div>
                  )}
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="p-4 border-t border-white/5 shrink-0 bg-base-300/30">
          <div className="modal-action mt-0 gap-2">
            <button
              onClick={onClose}
              className="btn btn-ghost hover:bg-white/5"
              disabled={isPreviewLoading}
            >
              Cancel
            </button>
            <button
              onClick={onConfirm}
              className="btn btn-warning shadow-lg shadow-warning/10 font-bold tracking-wide"
              disabled={isPreviewLoading}
            >
              Continue {'->'}
            </button>
          </div>
        </div>
      </div>
    </dialog>,
    document.body,
  );
}
