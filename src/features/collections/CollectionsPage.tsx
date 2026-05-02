/**
 * CollectionsPage — Thin layout component for the collections feature.
 *
 * **658 → ~120 lines** — All state derivation removed.
 * Backend computes: active collection, undo target, dirty state, signatures.
 * Frontend just renders: corridor tabs + list + preview.
 *
 * Replaces: 6 useMemo chains, 3 useEffect syncs, resolveActiveCollection,
 *           buildCollectionWorkspaceRows, findWorkspaceRowByCollectionId.
 */

import { useState, useCallback, useEffect, useMemo } from 'react';
import { Layers, Save } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useActiveGame } from '../../hooks/useActiveGame';
import { useSafeModeToggle } from './hooks/useSafeModeToggle';
import ModeSwitchConfirmModal from '../safe-mode/ModeSwitchConfirmModal';
import PinEntryModal from '../safe-mode/PinEntryModal';

import { useCorridor } from './hooks/useCorridor';
import {
  useCollections,
  useDeleteCollection,
  useUpdateCollection,
} from './hooks/useCollections';
import { CollectionList } from './components/CollectionList';
import { CollectionPreviewPanel } from './components/CollectionPreviewPanel';
import { SaveCollectionModal } from './components/SaveCollectionModal';
import { ApplyCollectionModal } from './components/ApplyCollectionModal';
import {
  buildCurrentRuntimeRow,
  CURRENT_RUNTIME_ROW_ID,
  isCollectionWorkspaceSourceEqual,
  type CollectionListRow,
  type CollectionSaveRequest,
  type CollectionWorkspaceSource,
} from './types';
import { getCollectionDisplayName, useUnsavedLabels } from '../../lib/corridorLabels';

export default function CollectionsPage() {
  const { t } = useTranslation(['collections', 'safe_mode']);
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

  const gameId = activeGame?.id ?? null;

  // ── v2 Queries ──
  const corridor = useCorridor(gameId, safeMode);
  const collections = useCollections(gameId, safeMode);

  // ── v2 Mutations ──
  const deleteMutation = useDeleteCollection();
  const updateMutation = useUpdateCollection();

  // ── Local UI State ──
  const [selectedSource, setSelectedSource] = useState<CollectionWorkspaceSource | null>(null);
  const [saveModalOpen, setSaveModalOpen] = useState(false);
  const [saveRequest, setSaveRequest] = useState<CollectionSaveRequest | null>(null);
  const [applyTargetId, setApplyTargetId] = useState<string | null>(null);

  // Reset selection when corridor changes so stale cross-corridor IDs don't cause failed preview queries
  useEffect(() => {
    setSelectedSource(null);
    setApplyTargetId(null);
    setSaveRequest(null);
  }, [safeMode]);

  const unsavedLabels = useUnsavedLabels();

  const rows = useMemo<CollectionListRow[]>(() => {
    const collectionRows: CollectionListRow[] = (collections.data ?? []).map((collection) => ({
      kind: 'stored_collection',
      rowId: collection.id,
      collection,
    }));
    if (!corridor.data?.is_dirty) {
      return collectionRows;
    }

    return [
      buildCurrentRuntimeRow(
        corridor.data,
        getCollectionDisplayName({
          name: null,
          isUnsaved: true,
          isSafe: corridor.data.is_safe,
          labels: unsavedLabels,
        }),
      ),
      ...collectionRows,
    ];
  }, [collections.data, corridor.data, unsavedLabels]);

  const effectiveSource = useMemo<CollectionWorkspaceSource | null>(() => {
    const hasCurrentRuntime = rows.some((row) => row.kind === 'current_runtime');
    const hasStoredCollection = (collectionId: string) =>
      rows.some((row) => row.kind === 'stored_collection' && row.collection.id === collectionId);

    if (selectedSource) {
      if (selectedSource.kind === 'current_runtime' && hasCurrentRuntime) {
        return selectedSource;
      }
      if (
        selectedSource.kind === 'stored_collection' &&
        hasStoredCollection(selectedSource.collectionId)
      ) {
        return selectedSource;
      }
    }

    const activeCollectionId = corridor.data?.active_collection_id;
    if (activeCollectionId && hasStoredCollection(activeCollectionId)) {
      return { kind: 'stored_collection', collectionId: activeCollectionId };
    }

    if (corridor.data?.is_dirty && hasCurrentRuntime) {
      return { kind: 'current_runtime' };
    }

    const storedUnsaved = rows.find(
      (row) => row.kind === 'stored_collection' && row.collection.is_unsaved,
    );
    if (storedUnsaved && storedUnsaved.kind === 'stored_collection') {
      return { kind: 'stored_collection', collectionId: storedUnsaved.collection.id };
    }

    const firstStored = rows.find((row) => row.kind === 'stored_collection');
    if (firstStored && firstStored.kind === 'stored_collection') {
      return { kind: 'stored_collection', collectionId: firstStored.collection.id };
    }

    if (hasCurrentRuntime) {
      return { kind: 'current_runtime' };
    }

    return null;
  }, [corridor.data, rows, selectedSource]);

  useEffect(() => {
    if (isCollectionWorkspaceSourceEqual(selectedSource, effectiveSource)) {
      return;
    }
    setSelectedSource(effectiveSource);
  }, [effectiveSource, selectedSource]);

  const effectiveSelectedId = effectiveSource
    ? effectiveSource.kind === 'current_runtime'
      ? CURRENT_RUNTIME_ROW_ID
      : effectiveSource.collectionId
    : null;

  // ── Handlers ──
  const handleSelect = useCallback((rowId: string) => {
    if (rowId === CURRENT_RUNTIME_ROW_ID) {
      setSelectedSource({ kind: 'current_runtime' });
      return;
    }

    setSelectedSource({ kind: 'stored_collection', collectionId: rowId });
  }, []);

  const handleApply = useCallback(
    (collectionId: string, _name: string) => {
      if (!gameId) return;
      setSelectedSource({ kind: 'stored_collection', collectionId });
      setApplyTargetId(collectionId);
    },
    [gameId],
  );

  const handleDelete = useCallback(
    (collectionId: string) => {
      if (!gameId) return;
      deleteMutation.mutate({ gameId, id: collectionId });
      setSelectedSource((current) => {
        if (current?.kind !== 'stored_collection' || current.collectionId !== collectionId) {
          return current;
        }

        return null;
      });
    },
    [gameId, deleteMutation],
  );

  const handleRename = useCallback(
    (collectionId: string, newName: string) => {
      if (!gameId) return;
      updateMutation.mutate({ gameId, id: collectionId, name: newName });
    },
    [gameId, updateMutation],
  );

  const handleCorridorTabSwitch = useCallback(
    async (targetSafeMode: boolean) => {
      if (safeMode === targetSafeMode) return;
      setSelectedSource(null);
      await toggleSafeMode();
    },
    [safeMode, toggleSafeMode],
  );

  const handleSave = useCallback((request: CollectionSaveRequest) => {
    setSaveRequest(request);
    setSaveModalOpen(true);
  }, []);

  // ── No active game guard ──
  if (!activeGame) {
    return (
      <div className="h-full flex items-center justify-center">
        <div className="text-center p-6 rounded-xl bg-base-200 border border-base-300">
          <h2 className="text-lg font-semibold">{t('collections:page.no_active_game.title')}</h2>
          <p className="text-sm text-base-content/70 mt-2">
            {t('collections:page.no_active_game.desc')}
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="h-full p-4 md:p-6 bg-base-100/50 flex flex-col w-full max-w-screen-2xl mx-auto">
      {/* Header */}
      <div className="mb-6 shrink-0 flex flex-col sm:flex-row sm:items-start justify-between gap-4">
        <div>
          <h1 className="text-3xl font-bold tracking-tight flex items-center gap-3">
            <Layers size={28} className="text-primary" />
            {t('collections:page.title')}
          </h1>
          <p className="text-base-content/60 mt-2 max-w-2xl text-sm">
            {t('collections:page.desc')}
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
            {t('collections:tab.safe')}
          </button>
          <button
            className={`tab tab-sm flex-1 transition-colors ${!safeMode ? 'tab-active bg-error/20 text-error rounded-md! font-medium shadow-sm' : 'text-base-content/60 hover:text-base-content'}`}
            onClick={() => {
              void handleCorridorTabSwitch(false);
            }}
          >
            {t('collections:tab.unsafe')}
          </button>
        </div>

        <div className="flex items-center gap-2">
          <button className="btn btn-secondary btn-sm" onClick={() => setSaveModalOpen(true)}>
            <Save size={14} />
            {t('collections:page.actions.save_current')}
          </button>
        </div>
      </div>

      {/* Main Grid */}
      <div className="grid grid-cols-1 lg:grid-cols-12 gap-6 flex-1 min-h-0">
        {/* LEFT: Collection List */}
        <div className="lg:col-span-8 flex flex-col">
          <div className="card bg-base-200/30 border border-base-content/5 shadow-lg flex-1 flex flex-col transition-all duration-300 overflow-hidden">
            <div className="card-body p-0 flex-1 overflow-y-auto custom-scrollbar relative min-h-75">
              <CollectionList
                rows={rows}
                selectedId={effectiveSelectedId}
                isLoading={collections.isLoading}
                safeMode={safeMode}
                onSelect={handleSelect}
                onApply={handleApply}
                onDelete={handleDelete}
                onRename={handleRename}
                onSave={handleSave}
                isApplying={!!applyTargetId}
                isDeleting={deleteMutation.isPending}
              />
            </div>
          </div>
        </div>

        {/* RIGHT: Preview Panel */}
        <div className="lg:col-span-4 flex flex-col min-h-0 bg-base-200/30 rounded-2xl border border-base-content/5 overflow-hidden shadow-lg">
          <CollectionPreviewPanel
            source={effectiveSource}
            gameId={gameId}
            corridorSnapshot={corridor.data}
          />
        </div>
      </div>

      {/* Corridor Switch Modals */}
      <ModeSwitchConfirmModal
        open={confirmModalOpen}
        targetSafeMode={confirmTargetSafeMode}
        onClose={closeConfirmModal}
        onConfirm={handleConfirmSwitch}
      />
      <PinEntryModal
        open={pinModalOpen}
        onClose={closePinModal}
        onSuccess={async () => {
          handlePinSuccess();
        }}
      />

      {saveModalOpen && (
        <SaveCollectionModal
          onClose={() => {
            setSaveModalOpen(false);
            setSaveRequest(null);
          }}
          saveMode={saveRequest?.mode}
          sourceCollectionId={saveRequest?.sourceCollectionId ?? null}
          onSaved={(collectionId) => {
            setSelectedSource({ kind: 'stored_collection', collectionId });
          }}
        />
      )}
      {applyTargetId && (
        <ApplyCollectionModal
          collectionId={applyTargetId}
          onClose={() => setApplyTargetId(null)}
        />
      )}
    </div>
  );
}
