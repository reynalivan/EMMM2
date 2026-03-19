import { useMemo, useState } from 'react';
import { X, Save, Loader2, Package } from 'lucide-react';
import { useQueryClient } from '@tanstack/react-query';
import { useActiveGame } from '../../../hooks/useActiveGame';
import { useAppStore } from '../../../stores/useAppStore';
import { toast, useToastStore } from '../../../stores/useToastStore';
import { ALL_DISABLED_LABEL } from '../../../lib/corridorLabels';
import { ModGroupList } from './ModGroupList';
import { buildGroupedModsWithObjectStates } from '../utils/groupMods';
import {
  useCollectionRuntimePreview,
  useCorridorRuntimeSnapshot,
  useSaveCurrentAsCollection,
  useSaveSnapshotCollectionAsNamed,
} from '../hooks/useCollections';
import { invalidateCorridorRuntime } from '../utils/invalidateCorridorRuntime';
import { refetchCollectionRuntime } from '../utils/refetchCollectionRuntime';
import type { SaveCollectionMode } from '../../../types/collection';

interface SaveCollectionModalProps {
  mode: SaveCollectionMode;
  sourceCollectionId?: string;
  sourceCollectionName?: string;
  onSaved?: (collectionId: string) => void;
  onClose: () => void;
}

function buildDefaultCollectionName(): string {
  const now = new Date();
  const yyyy = now.getFullYear();
  const mm = String(now.getMonth() + 1).padStart(2, '0');
  const dd = String(now.getDate()).padStart(2, '0');
  const hh = String(now.getHours()).padStart(2, '0');
  const min = String(now.getMinutes()).padStart(2, '0');
  return `Unsaved ${yyyy}${mm}${dd}${hh}${min}`;
}

export default function SaveCollectionModal({
  mode,
  sourceCollectionId,
  sourceCollectionName,
  onSaved,
  onClose,
}: SaveCollectionModalProps) {
  const { activeGame } = useActiveGame();
  const { safeMode, setWorkspaceSelectionForCorridor } = useAppStore();
  const queryClient = useQueryClient();
  const isSnapshotMode = mode === 'snapshot_collection';

  const [name, setName] = useState(buildDefaultCollectionName());
  const [isDisablingAll, setIsDisablingAll] = useState(false);

  const saveCurrentMutation = useSaveCurrentAsCollection();
  const saveSnapshotMutation = useSaveSnapshotCollectionAsNamed();
  const corridorRuntimeQuery = useCorridorRuntimeSnapshot(activeGame?.id ?? null, safeMode);
  const snapshotPreviewQuery = useCollectionRuntimePreview(
    sourceCollectionId ?? null,
    activeGame?.id ?? null,
  );

  const previewMods = isSnapshotMode
    ? (snapshotPreviewQuery.data?.roots ?? [])
    : (corridorRuntimeQuery.data?.roots ?? []);
  const previewObjectStates = isSnapshotMode
    ? (snapshotPreviewQuery.data?.object_states ?? [])
    : (corridorRuntimeQuery.data?.object_states ?? []);
  const isLoadingPreview = isSnapshotMode ? snapshotPreviewQuery.isLoading : corridorRuntimeQuery.isLoading;
  const isSaving = isSnapshotMode ? saveSnapshotMutation.isPending : saveCurrentMutation.isPending;
  const activeModCount = previewMods.length;

  const objectStates = useMemo(
    () =>
      (corridorRuntimeQuery.data?.object_states ?? []).map((object) => ({
        object_id: object.object_id,
        is_enabled: object.is_enabled,
      })),
    [corridorRuntimeQuery.data?.object_states],
  );

  const previewRelevantObjectIds = useMemo(() => {
    const relevantObjectIds = new Set<string>();
    previewMods.forEach((mod) => {
      if (mod.object_id) {
        relevantObjectIds.add(mod.object_id);
      }
    });
    previewObjectStates.forEach((state) => {
      if (!state.is_enabled) {
        relevantObjectIds.add(state.object_id);
      }
    });
    return relevantObjectIds;
  }, [previewMods, previewObjectStates]);

  const groupedPreviewMods = useMemo(
    () =>
      buildGroupedModsWithObjectStates(previewMods, previewObjectStates, {
        mode: 'preview',
        relevantObjectIds: previewRelevantObjectIds.size > 0 ? previewRelevantObjectIds : undefined,
      }),
    [previewMods, previewObjectStates, previewRelevantObjectIds],
  );

  const handleSave = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!activeGame || !name.trim()) {
      return;
    }

    if (isSnapshotMode) {
      if (!sourceCollectionId) {
        toast.error('No snapshot selected to save.');
        return;
      }

      const result = await saveSnapshotMutation.mutateAsync({
        source_collection_id: sourceCollectionId,
        game_id: activeGame.id,
        name: name.trim(),
      });
      setWorkspaceSelectionForCorridor(activeGame.id, safeMode, {
        kind: 'stored_collection',
        collection_id: result.collection.id,
      });
      onSaved?.(result.collection.id);
      onClose();
      return;
    }

    try {
      const result = await saveCurrentMutation.mutateAsync({
        name: name.trim(),
        game_id: activeGame.id,
        is_safe_context: safeMode,
        object_states: objectStates,
      });
      setWorkspaceSelectionForCorridor(activeGame.id, safeMode, {
        kind: 'stored_collection',
        collection_id: result.collection.id,
      });
      onSaved?.(result.collection.id);
      onClose();
    } catch (error) {
      toast.error(String(error));
    }
  };

  const handleDisableAll = async () => {
    if (!activeGame || isSnapshotMode || activeModCount === 0) {
      return;
    }

    setIsDisablingAll(true);
    const toastId = toast.info('Disabling all mods...', 0);

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const modIds = previewMods.filter((mod) => !mod.id.startsWith('nested_')).map((mod) => mod.id);

      await invoke('bulk_toggle_mods_by_ids', {
        modIds,
        enable: false,
        gameId: activeGame.id,
      });

      await invalidateCorridorRuntime(queryClient);
      queryClient.invalidateQueries({ queryKey: ['objects'] });
      queryClient.invalidateQueries({ queryKey: ['mod-folders'] });
      await refetchCollectionRuntime(queryClient, {
        gameId: activeGame.id,
        isSafe: safeMode,
      });

      useToastStore.getState().removeToast(toastId);
      toast.success('Cleared loadout.');
    } catch (error) {
      useToastStore.getState().removeToast(toastId);
      toast.error(String(error));
    } finally {
      setIsDisablingAll(false);
    }
  };

  const heading = isSnapshotMode ? 'Save Snapshot as Collection' : 'Save Current State';
  const description = isSnapshotMode
    ? `Save the selected snapshot${sourceCollectionName ? ` (${sourceCollectionName})` : ''} as a new named ${safeMode ? 'Safe' : 'Unsafe'} collection.`
    : `Snapshots the current ${safeMode ? 'Safe' : 'Unsafe'} corridor state into a new named collection.`;
  const privacyHint = isSnapshotMode
    ? 'The new collection keeps the selected snapshot context.'
    : `To save a ${safeMode ? 'Unsafe' : 'Safe'} collection, close this and switch tabs.`;

  return (
    <div className="fixed inset-0 z-100 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4">
      <div className="card bg-base-200 border border-white/10 shadow-2xl w-full max-w-md my-auto animate-in fade-in zoom-in-95 duration-200">
        <div className="card-body p-6">
          <div className="flex items-center justify-between mb-2">
            <h2 className="card-title text-xl flex gap-2 items-center">
              <Save size={20} className="text-secondary" />
              {heading}
            </h2>
            <button
              className="btn btn-sm btn-circle btn-ghost"
              onClick={onClose}
              disabled={isDisablingAll || isSaving}
            >
              <X size={16} />
            </button>
          </div>

          <p className="text-sm text-base-content/60 mb-6">{description}</p>

          <form onSubmit={handleSave} className="space-y-5">
            <div className="form-control">
              <label className="label py-1">
                <span className="label-text font-medium text-base-content/80">Collection Name</span>
              </label>
              <input
                className="input input-bordered focus:border-secondary bg-base-300 w-full"
                placeholder="e.g. Abyss Run 1"
                value={name}
                onChange={(event) => setName(event.target.value)}
                autoFocus
                required
              />
            </div>

            <div className="form-control bg-base-300/50 p-3 rounded-lg border border-white/5 flex flex-row items-center justify-between">
              <div>
                <p className="text-xs text-base-content/70 font-medium">Privacy Context</p>
                <p className={`text-sm font-bold ${safeMode ? 'text-success' : 'text-error'}`}>
                  {safeMode ? 'SAFE' : 'UNSAFE'}
                </p>
              </div>
              <div className="text-right max-w-30">
                <p className="text-[10px] text-base-content/50 leading-tight">{privacyHint}</p>
              </div>
            </div>

            <div className="border border-white/5 rounded-xl bg-base-300/20 overflow-hidden">
              <div className="flex items-center justify-between bg-base-300/50 p-3 border-b border-white/5">
                <h3 className="text-xs font-bold text-base-content/70 uppercase tracking-wider flex items-center gap-1.5">
                  <Package size={14} />
                  {isSnapshotMode ? 'Snapshot Mods' : 'Current Mods'} ({activeModCount})
                </h3>
                {!isSnapshotMode && activeModCount > 0 && (
                  <button
                    type="button"
                    onClick={handleDisableAll}
                    disabled={isDisablingAll || isSaving}
                    className="btn btn-xs btn-outline btn-error opacity-80 hover:opacity-100"
                  >
                    {isDisablingAll ? <Loader2 size={12} className="animate-spin" /> : 'Disable All'}
                  </button>
                )}
              </div>

              <div className="p-2">
                {isLoadingPreview ? (
                  <div className="flex justify-center py-6 text-base-content/40">
                    <Loader2 size={20} className="animate-spin" />
                  </div>
                ) : groupedPreviewMods.length === 0 ? (
                  <p className="text-xs text-center py-6 text-base-content/40 italic">
                    {isSnapshotMode
                      ? 'Selected snapshot has no stored mods.'
                      : `No enabled main mods. Saving now will create an ${ALL_DISABLED_LABEL} snapshot.`}
                  </p>
                ) : (
                  <div className="max-h-48 overflow-y-auto custom-scrollbar pr-2 pl-1 py-1">
                    <ModGroupList
                      groups={groupedPreviewMods}
                      colorClass="text-secondary/80"
                      emptyGroupMessage="No mods in this object."
                      emptyStateMessage={
                        isSnapshotMode
                          ? 'Selected snapshot has no stored mods.'
                          : `No enabled main mods. Saving now will create an ${ALL_DISABLED_LABEL} snapshot.`
                      }
                      resetKey={`save-modal-${mode}-${sourceCollectionId ?? 'current'}`}
                    />
                  </div>
                )}
              </div>
            </div>

            <div className="pt-2">
              <button
                type="submit"
                disabled={!name.trim() || isSaving || isDisablingAll}
                className="btn btn-secondary w-full"
              >
                {isSaving ? (
                  <>
                    <Loader2 size={18} className="animate-spin" /> Saving...
                  </>
                ) : isSnapshotMode ? (
                  'Save Snapshot as Collection'
                ) : (
                  'Save Collection'
                )}
              </button>
            </div>
          </form>
        </div>
      </div>
    </div>
  );
}
