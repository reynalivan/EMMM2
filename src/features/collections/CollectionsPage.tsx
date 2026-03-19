import { useState, useCallback, useEffect, useMemo, useRef } from 'react';
import { Layers, Trash2, Edit2, Check, X, Save, Loader2 } from 'lucide-react';
import { useActiveGame } from '../../hooks/useActiveGame';
import {
  useApplyCollection,
  useCollectionRuntimePreview,
  useCollections,
  useCorridorRuntimeSnapshot,
  useDeleteCollection,
  useUpdateCollection,
} from './hooks/useCollections';
import type { Collection, CollectionObjectState, SaveCollectionMode } from '../../types/collection';
import CollectionWorkspace from './components/CollectionWorkspace';
import ApplyCollectionModal from './components/ApplyCollectionModal';
import SaveCollectionModal from './components/SaveCollectionModal';
import { useSafeModeToggle } from '../../hooks/useSafeModeToggle';
import ModeSwitchConfirmModal from '../safe-mode/ModeSwitchConfirmModal';
import PinEntryModal from '../safe-mode/PinEntryModal';
import { toast } from '../../stores/useToastStore';
import {
  getWorkspaceSelectionForCorridor,
  type CollectionWorkspaceSource,
} from '../../lib/corridorSelection';
import { useAppStore } from '../../stores/useAppStore';
import {
  areWorkspaceSourcesEqual,
  buildCollectionWorkspaceRows,
  findWorkspaceRow,
  isWorkspaceSourceAvailable,
  resolvePreferredWorkspaceSource,
  type CollectionWorkspaceRow,
} from './utils/workspaceSelection';

interface SaveModalState {
  mode: SaveCollectionMode;
  sourceCollectionId?: string;
  sourceCollectionName?: string;
}

export default function CollectionsPage() {
  const { activeGame } = useActiveGame();
  const {
    toggleSafeMode,
    handleConfirmSwitch,
    handlePinSuccess,
    confirmModalOpen,
    confirmTargetSafeMode,
    closeConfirmModal,
    pinModalOpen,
    closePinModal,
    safeMode,
  } = useSafeModeToggle();

  const [saveModalState, setSaveModalState] = useState<SaveModalState | null>(null);
  const [pendingWorkspaceSource, setPendingWorkspaceSource] =
    useState<CollectionWorkspaceSource | null>(null);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editName, setEditName] = useState('');
  const [, setWorkspaceDraftObjectStates] = useState<CollectionObjectState[]>([]);
  const [, setWorkspaceHasObjectStateChanges] = useState(false);
  const workspaceDraftObjectStatesRef = useRef<CollectionObjectState[]>([]);
  const workspaceHasObjectStateChangesRef = useRef(false);
  const [confirmApply, setConfirmApply] = useState<{
    id: string;
    name: string;
  } | null>(null);

  const collectionsQuery = useCollections(activeGame?.id ?? null);
  const updateMutation = useUpdateCollection();
  const deleteMutation = useDeleteCollection();
  const applyMutation = useApplyCollection();
  const corridorRuntimeQuery = useCorridorRuntimeSnapshot(activeGame?.id ?? null, safeMode);
  const {
    workspaceSelectionByCorridor,
    setWorkspaceSelectionForCorridor,
    clearWorkspaceSelectionForCorridor,
  } = useAppStore();

  const persistedWorkspaceSource = useMemo(
    () => getWorkspaceSelectionForCorridor(workspaceSelectionByCorridor, activeGame?.id, safeMode),
    [activeGame?.id, safeMode, workspaceSelectionByCorridor],
  );

  const corridorCollections = useMemo(
    () => collectionsQuery.data?.filter((c) => c.is_safe_context === safeMode) ?? [],
    [collectionsQuery.data, safeMode],
  );

  const workspaceRows = useMemo(
    () =>
      buildCollectionWorkspaceRows(
        activeGame?.id,
        safeMode,
        corridorCollections,
        corridorRuntimeQuery.data,
      ),
    [activeGame?.id, corridorCollections, corridorRuntimeQuery.data, safeMode],
  );

  const resolvedWorkspaceSource = useMemo(
    () => {
      if (
        pendingWorkspaceSource &&
        !isWorkspaceSourceAvailable(workspaceRows, pendingWorkspaceSource)
      ) {
        return pendingWorkspaceSource;
      }

      return resolvePreferredWorkspaceSource(
        workspaceRows,
        persistedWorkspaceSource,
        corridorRuntimeQuery.data,
      );
    },
    [
      corridorRuntimeQuery.data,
      pendingWorkspaceSource,
      persistedWorkspaceSource,
      workspaceRows,
    ],
  );

  const selectedWorkspaceRow = useMemo(
    () => findWorkspaceRow(workspaceRows, resolvedWorkspaceSource),
    [resolvedWorkspaceSource, workspaceRows],
  );

  const selectedStoredCollectionId =
    resolvedWorkspaceSource?.kind === 'stored_collection'
      ? resolvedWorkspaceSource.collection_id
      : null;

  const selectedCollectionPreview = useCollectionRuntimePreview(
    selectedStoredCollectionId,
    activeGame?.id ?? null,
  );

  const selectedWorkspaceObjectStates = useMemo(() => {
    const sourceStates =
      resolvedWorkspaceSource?.kind === 'current_runtime'
        ? (corridorRuntimeQuery.data?.object_states ?? [])
        : (selectedCollectionPreview.data?.object_states ?? []);
    return sourceStates.map((state) => ({
      object_id: state.object_id,
      is_enabled: state.is_enabled,
      name: state.name,
      object_type: state.object_type,
    }));
  }, [
    corridorRuntimeQuery.data?.object_states,
    resolvedWorkspaceSource?.kind,
    selectedCollectionPreview.data?.object_states,
  ]);

  const selectedPreviewRoots = useMemo(() => {
    if (resolvedWorkspaceSource?.kind === 'current_runtime') {
      return corridorRuntimeQuery.data?.roots ?? [];
    }
    return selectedCollectionPreview.data?.roots ?? [];
  }, [corridorRuntimeQuery.data?.roots, resolvedWorkspaceSource?.kind, selectedCollectionPreview.data?.roots]);

  const isSelectedPreviewLoading =
    resolvedWorkspaceSource?.kind === 'current_runtime'
      ? corridorRuntimeQuery.isLoading
      : selectedCollectionPreview.isLoading;

  useEffect(() => {
    setWorkspaceDraftObjectStates([]);
    setWorkspaceHasObjectStateChanges(false);
    workspaceDraftObjectStatesRef.current = [];
    workspaceHasObjectStateChangesRef.current = false;
  }, [
    resolvedWorkspaceSource?.kind,
    resolvedWorkspaceSource?.kind === 'stored_collection'
      ? resolvedWorkspaceSource.collection_id
      : null,
  ]);

  useEffect(() => {
    if (
      pendingWorkspaceSource &&
      isWorkspaceSourceAvailable(workspaceRows, pendingWorkspaceSource)
    ) {
      setPendingWorkspaceSource(null);
    }
  }, [pendingWorkspaceSource, workspaceRows]);

  useEffect(() => {
    if (!activeGame) {
      return;
    }
    if (!collectionsQuery.isSuccess) {
      return;
    }
    if (collectionsQuery.isFetching) {
      return;
    }
    if (
      pendingWorkspaceSource &&
      !isWorkspaceSourceAvailable(workspaceRows, pendingWorkspaceSource)
    ) {
      return;
    }
    if (!resolvedWorkspaceSource) {
      if (persistedWorkspaceSource) {
        clearWorkspaceSelectionForCorridor(activeGame.id, safeMode);
      }
      return;
    }

    if (areWorkspaceSourcesEqual(persistedWorkspaceSource, resolvedWorkspaceSource)) {
      return;
    }

    setWorkspaceSelectionForCorridor(activeGame.id, safeMode, resolvedWorkspaceSource);
  }, [
    activeGame,
    collectionsQuery.isFetching,
    collectionsQuery.isSuccess,
    clearWorkspaceSelectionForCorridor,
    persistedWorkspaceSource,
    pendingWorkspaceSource,
    resolvedWorkspaceSource,
    safeMode,
    setWorkspaceSelectionForCorridor,
    workspaceRows,
  ]);

  const saveObjectStates = useCallback(
    async (collection: Collection, objectStates: CollectionObjectState[]): Promise<boolean> => {
      if (!activeGame) return false;

      try {
        await updateMutation.mutateAsync({
          id: collection.id,
          game_id: activeGame.id,
          object_states: objectStates.map(({ object_id, is_enabled }) => ({
            object_id,
            is_enabled,
          })),
        });
        toast.success(`Updated object states for ${collection.name}`);
        return true;
      } catch (error) {
        toast.error(String(error));
        return false;
      }
    },
    [activeGame, updateMutation],
  );

  const autoSaveWorkspaceIfNeeded = useCallback(async (): Promise<boolean> => {
    if (!workspaceHasObjectStateChangesRef.current) {
      return true;
    }

    if (selectedWorkspaceRow?.sourceKind !== 'named_collection') {
      return true;
    }

    if (workspaceDraftObjectStatesRef.current.length === 0) {
      return true;
    }

    const success = await saveObjectStates(
      selectedWorkspaceRow.collection,
      workspaceDraftObjectStatesRef.current,
    );
    if (!success) {
      toast.error('Failed to auto-save workspace changes. Switch is cancelled.');
      return false;
    }

    return true;
  }, [
    saveObjectStates,
    selectedWorkspaceRow,
  ]);

  const handleApply = async (collection: Collection, skipWorkspaceSave: boolean) => {
    if (
      !skipWorkspaceSave &&
      selectedWorkspaceRow?.source.kind === 'stored_collection' &&
      selectedWorkspaceRow.collection.id === collection.id
    ) {
      const saved = await autoSaveWorkspaceIfNeeded();
      if (!saved) {
        return;
      }
    }

    if (activeGame) {
      setWorkspaceSelectionForCorridor(activeGame.id, safeMode, {
        kind: 'stored_collection',
        collection_id: collection.id,
      });
    }
    setConfirmApply({ id: collection.id, name: collection.name });
  };

  const startEdit = (collection: Collection) => {
    setEditingId(collection.id);
    setEditName(collection.name);
  };

  const cancelEdit = () => {
    setEditingId(null);
    setEditName('');
  };

  const saveEdit = async (collection: Collection) => {
    if (!activeGame) {
      return;
    }

    if (!editName.trim() || editName.trim() === collection.name) {
      cancelEdit();
      return;
    }
    await updateMutation.mutateAsync({
      id: collection.id,
      game_id: activeGame.id,
      name: editName.trim(),
    });
    setEditingId(null);
  };

  const handleCollectionSelect = useCallback(
    async (nextSource: CollectionWorkspaceSource) => {
      if (areWorkspaceSourcesEqual(nextSource, resolvedWorkspaceSource)) {
        return;
      }

      const saved = await autoSaveWorkspaceIfNeeded();
      if (!saved) {
        return;
      }

      if (activeGame) {
        setWorkspaceSelectionForCorridor(activeGame.id, safeMode, nextSource);
      }
    },
    [
      activeGame,
      autoSaveWorkspaceIfNeeded,
      resolvedWorkspaceSource,
      safeMode,
      setWorkspaceSelectionForCorridor,
    ],
  );

  const handleCorridorTabSwitch = useCallback(
    async (targetSafeMode: boolean) => {
      if (safeMode === targetSafeMode) {
        return;
      }

      const saved = await autoSaveWorkspaceIfNeeded();
      if (!saved) {
        return;
      }

      await toggleSafeMode();
      setWorkspaceDraftObjectStates([]);
      setWorkspaceHasObjectStateChanges(false);
      workspaceDraftObjectStatesRef.current = [];
      workspaceHasObjectStateChangesRef.current = false;
    },
    [safeMode, autoSaveWorkspaceIfNeeded, toggleSafeMode],
  );

  const handleWorkspaceStateChange = useCallback(
    (draftStates: CollectionObjectState[], hasChanges: boolean) => {
      workspaceDraftObjectStatesRef.current = draftStates;
      workspaceHasObjectStateChangesRef.current = hasChanges;
      setWorkspaceDraftObjectStates(draftStates);
      setWorkspaceHasObjectStateChanges(hasChanges);
    },
    [],
  );

  const openCurrentStateSaveModal = useCallback(() => {
    setSaveModalState({ mode: 'current_state' });
  }, []);

  const openSnapshotSaveModal = useCallback((collection: Collection) => {
    setSaveModalState({
      mode: 'snapshot_collection',
      sourceCollectionId: collection.id,
      sourceCollectionName: collection.name,
    });
  }, []);

  const handleRowPrimaryAction = useCallback(
    async (row: CollectionWorkspaceRow) => {
      const saved = await autoSaveWorkspaceIfNeeded();
      if (!saved) {
        return;
      }

      if (activeGame) {
        setWorkspaceSelectionForCorridor(activeGame.id, safeMode, row.source);
      }

      if (row.primaryActionKind === 'save_current') {
        openCurrentStateSaveModal();
        return;
      }

      if (row.primaryActionKind === 'save_snapshot') {
        openSnapshotSaveModal(row.collection);
        return;
      }

      await handleApply(row.collection, true);
    },
    [
      activeGame,
      autoSaveWorkspaceIfNeeded,
      handleApply,
      openCurrentStateSaveModal,
      openSnapshotSaveModal,
      safeMode,
      setWorkspaceSelectionForCorridor,
    ],
  );

  if (!activeGame) {
    return (
      <div className="h-full flex items-center justify-center">
        <div className="text-center p-6 rounded-xl bg-base-200 border border-base-300">
          <h2 className="text-lg font-semibold">No Active Game</h2>
          <p className="text-sm text-base-content/70 mt-2">
            Select a game first, then open Collections.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="h-full p-4 md:p-6 bg-base-100/50 flex flex-col w-full max-w-screen-2xl mx-auto">
      <div className="mb-6 shrink-0 flex flex-col sm:flex-row sm:items-start justify-between gap-4">
        <div>
          <h1 className="text-3xl font-bold tracking-tight flex items-center gap-3">
            <Layers size={28} className="text-primary" />
            Collections
          </h1>
          <p className="text-base-content/60 mt-2 max-w-2xl text-sm">
            Save your currently enabled mods and variants as a named collection. Apply collections
            atomically to instantly switch your entire game loadout.
          </p>
        </div>

        {/* SAFE/UNSAFE Tabs */}
        <div className="tabs tabs-boxed bg-base-200/50 p-1.5 gap-1 w-full sm:w-auto shrink-0 min-w-70 shadow-sm">
          <button
            className={`tab tab-sm flex-1 transition-colors ${safeMode ? 'tab-active bg-success/20 text-success rounded-md! font-medium shadow-sm' : 'text-base-content/60 hover:text-base-content'}`}
            onClick={() => {
              void handleCorridorTabSwitch(true);
            }}
          >
            SAFE
          </button>
          <button
            className={`tab tab-sm flex-1 transition-colors ${!safeMode ? 'tab-active bg-error/20 text-error rounded-md! font-medium shadow-sm' : 'text-base-content/60 hover:text-base-content'}`}
            onClick={() => {
              void handleCorridorTabSwitch(false);
            }}
          >
            UNSAFE
          </button>
        </div>

        <button className="btn btn-secondary btn-sm" onClick={openCurrentStateSaveModal}>
          <Save size={14} />
          Save Current State
        </button>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-12 gap-6 flex-1 min-h-0">
        {/* LEFT COLUMN: Collections List */}
        <div className="lg:col-span-8 flex flex-col">
          <div className="card bg-base-200/30 border border-white/5 shadow-lg flex-1 flex flex-col transition-all duration-300 overflow-hidden">
            <div className="card-body p-0 flex-1 overflow-y-auto custom-scrollbar relative min-h-75">
              {collectionsQuery.isLoading ? (
                <div className="flex items-center justify-center h-full text-base-content/50">
                  <Loader2 size={24} className="animate-spin" />
                </div>
              ) : (
                (() => {
                  if (workspaceRows.length === 0) {
                    return (
                      <div className="flex flex-col items-center justify-center p-8 text-center h-full absolute inset-0">
                        <div className="w-16 h-16 rounded-full bg-base-300 flex items-center justify-center mb-4 text-base-content/30 mt-8">
                          <Layers size={32} />
                        </div>
                        <h3 className="text-lg font-medium opacity-80 mb-2">
                          No {safeMode ? 'Safe' : 'Unsafe'} collections found
                        </h3>
                        <p className="text-sm opacity-50 max-w-sm">
                          Create your first {safeMode ? 'safe' : 'unsafe'} collection by clicking
                          "Save Current State" at the top right.
                        </p>
                      </div>
                    );
                  }

                  return (
                    <table className="table table-auto w-full">
                      <thead className="sticky top-0 bg-base-200/95 backdrop-blur z-10 border-b border-white/5 shadow-sm">
                        <tr className="border-none text-base-content/50">
                          <th className="w-1/2">Name</th>
                          <th>Mods</th>
                          <th className="text-right pr-6">Actions</th>
                        </tr>
                      </thead>
                      <tbody>
                        {workspaceRows.map((row) => {
                          const isSelected = areWorkspaceSourcesEqual(
                            row.source,
                            resolvedWorkspaceSource,
                          );
                          const badgeLabel =
                            row.sourceKind === 'current_runtime'
                              ? 'Current'
                              : row.sourceKind === 'stored_unsaved_snapshot'
                                ? 'Last Unsaved'
                                : null;
                          const modCount = isSelected
                            ? selectedPreviewRoots.length
                            : row.source.kind === 'current_runtime'
                              ? (corridorRuntimeQuery.data?.roots.length ?? row.collection.member_count)
                              : row.collection.member_count;

                          return (
                          <tr
                            key={row.collection.id}
                            onClick={() => {
                              void handleCollectionSelect(row.source);
                            }}
                            className={`hover border-white/5 transition-colors group cursor-pointer ${
                              isSelected
                                ? 'bg-primary/10 border-l-2 border-l-primary'
                                : ''
                            }`}
                          >
                            <td className="pl-4">
                              {editingId === row.collection.id ? (
                                <div className="flex items-center gap-2">
                                  <input
                                    type="text"
                                    className="input input-sm input-bordered w-full max-w-30"
                                    value={editName}
                                    onChange={(e) => setEditName(e.target.value)}
                                    onClick={(e) => e.stopPropagation()}
                                    autoFocus
                                    onKeyDown={(e) => {
                                      if (e.key === 'Enter') saveEdit(row.collection);
                                      if (e.key === 'Escape') cancelEdit();
                                    }}
                                  />
                                  <button
                                    className="btn btn-xs btn-square btn-success text-white shrink-0"
                                    onClick={(e) => {
                                      e.stopPropagation();
                                      saveEdit(row.collection);
                                    }}
                                  >
                                    <Check size={14} />
                                  </button>
                                  <button
                                    className="btn btn-xs btn-square btn-ghost shrink-0"
                                    onClick={(e) => {
                                      e.stopPropagation();
                                      cancelEdit();
                                    }}
                                  >
                                    <X size={14} />
                                  </button>
                                </div>
                              ) : (
                                <div className="font-medium text-[15px] flex items-center gap-2">
                                  <span className="truncate max-w-30 2xl:max-w-50">
                                    {row.collection.name}
                                  </span>
                                  {badgeLabel && (
                                    <span className="badge badge-sm badge-warning opacity-90 text-[10px] py-0 h-4 uppercase font-bold tracking-wider shrink-0">
                                      {badgeLabel}
                                    </span>
                                  )}
                                  {row.sourceKind === 'named_collection' && (
                                    <button
                                      className="btn btn-xs btn-square btn-ghost opacity-0 group-hover:opacity-100 transition-opacity text-base-content/40 hover:text-white shrink-0"
                                      onClick={(e) => {
                                        e.stopPropagation();
                                        startEdit(row.collection);
                                      }}
                                      title="Rename"
                                    >
                                      <Edit2 size={12} />
                                    </button>
                                  )}
                                </div>
                              )}
                            </td>
                            <td>
                              <span className="badge badge-sm badge-ghost opacity-70 shrink-0">
                                {modCount} mods
                              </span>
                            </td>

                            <td className="text-right pr-4">
                              <div className="flex items-center justify-end gap-2">
                                <button
                                  className={`btn btn-sm ${row.primaryActionKind === 'apply' ? 'btn-primary' : 'btn-secondary'}`}
                                  onClick={(e) => {
                                    e.stopPropagation();
                                    void handleRowPrimaryAction(row);
                                  }}
                                  disabled={
                                    row.primaryActionKind === 'apply'
                                      ? applyMutation.isPending || row.collection.member_count === 0
                                      : false
                                  }
                                >
                                  {row.primaryActionKind === 'save_current' ? (
                                    <>
                                      <Save size={14} className="mr-1" />
                                      Save Current
                                    </>
                                  ) : row.primaryActionKind === 'save_snapshot' ? (
                                    <>
                                      <Save size={14} className="mr-1" />
                                      Save As...
                                    </>
                                  ) : (
                                    'Apply'
                                  )}
                                </button>
                                {row.sourceKind === 'named_collection' && (
                                  <button
                                    className="btn btn-sm btn-square btn-ghost text-error/70 hover:text-error hover:bg-error/10 shrink-0"
                                    onClick={(e) => {
                                      e.stopPropagation();
                                      deleteMutation.mutate({
                                        id: row.collection.id,
                                        gameId: activeGame.id,
                                      });
                                      if (
                                        resolvedWorkspaceSource?.kind === 'stored_collection' &&
                                        resolvedWorkspaceSource.collection_id === row.collection.id
                                      ) {
                                        clearWorkspaceSelectionForCorridor(activeGame.id, safeMode);
                                      }
                                    }}
                                    disabled={deleteMutation.isPending}
                                    title="Delete collection"
                                  >
                                    <Trash2 size={16} />
                                  </button>
                                )}
                              </div>
                            </td>
                          </tr>
                          );
                        })}
                      </tbody>
                    </table>
                  );
                })()
              )}
            </div>
          </div>
        </div>

        {/* RIGHT COLUMN: Collection Preview Sidebar */}
        <div className="lg:col-span-4 flex flex-col min-h-0 bg-base-200/30 rounded-2xl border border-white/5 overflow-hidden shadow-lg">
          {selectedWorkspaceRow ? (
            <CollectionWorkspace
              collection={selectedWorkspaceRow.collection}
              sourceKind={selectedWorkspaceRow.sourceKind}
              primaryActionKind={selectedWorkspaceRow.primaryActionKind}
              previewRoots={selectedPreviewRoots}
              isPreviewLoading={isSelectedPreviewLoading}
              onPrimaryAction={(collection) => {
                if (selectedWorkspaceRow.primaryActionKind === 'save_snapshot') {
                  openSnapshotSaveModal(collection);
                  return;
                }
                if (selectedWorkspaceRow.primaryActionKind === 'save_current') {
                  openCurrentStateSaveModal();
                  return;
                }
                void handleApply(collection, false);
              }}
              isApplying={applyMutation.isPending}
              objectStates={selectedWorkspaceObjectStates}
              allowObjectStateEditing={selectedWorkspaceRow.sourceKind === 'named_collection'}
              isSavingObjectStates={updateMutation.isPending}
              onSaveObjectStates={(states) =>
                selectedWorkspaceRow.sourceKind === 'named_collection'
                  ? saveObjectStates(selectedWorkspaceRow.collection, states)
                  : Promise.resolve(false)
              }
              onWorkspaceStateChange={handleWorkspaceStateChange}
            />
          ) : (
            <div className="flex flex-col items-center justify-center p-8 text-center h-full">
              <div className="w-20 h-20 rounded-full bg-base-300 flex items-center justify-center mb-6 text-base-content/20 shadow-inner">
                <Layers size={40} className="opacity-50" />
              </div>
              <h3 className="text-xl font-bold opacity-80 mb-2">Collection Details</h3>
              <p className="text-base-content/50 max-w-sm leading-relaxed">
                Select a collection from the list on the left to view its contents, inspect mods,
                and manage your loadout.
              </p>
            </div>
          )}
        </div>
      </div>

      {/* Save Modal */}
      {saveModalState && (
        <SaveCollectionModal
          mode={saveModalState.mode}
          sourceCollectionId={saveModalState.sourceCollectionId}
          sourceCollectionName={saveModalState.sourceCollectionName}
          onSaved={(collectionId) => {
            setPendingWorkspaceSource({
              kind: 'stored_collection',
              collection_id: collectionId,
            });
          }}
          onClose={() => setSaveModalState(null)}
        />
      )}

      {/* Apply Confirmation Modal */}
      {confirmApply && (
        <ApplyCollectionModal
          collectionId={confirmApply.id}
          collectionName={confirmApply.name}
          onClose={() => setConfirmApply(null)}
        />
      )}

      {/* Confirmation Modal for Corridor Switch */}
      <ModeSwitchConfirmModal
        open={confirmModalOpen}
        targetSafeMode={confirmTargetSafeMode}
        onClose={closeConfirmModal}
        onConfirm={handleConfirmSwitch}
      />

      {/* Pin Entry Modal for Safe→Unsafe transition */}
      <PinEntryModal
        open={pinModalOpen}
        onClose={closePinModal}
        onSuccess={async () => {
          handlePinSuccess();
        }}
      />
    </div>
  );
}
