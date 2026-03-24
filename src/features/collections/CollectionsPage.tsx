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

import { useState, useCallback, useEffect } from 'react';
import { Layers, Save, Undo2 } from 'lucide-react';
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
  useUndoCollection,
} from './hooks/useCollections';
import { CollectionList } from './components/CollectionList';
import { CollectionPreviewPanel } from './components/CollectionPreviewPanel';
import { SaveCollectionModal } from './components/SaveCollectionModal';
import { ApplyCollectionModal } from './components/ApplyCollectionModal';

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
  const undoMutation = useUndoCollection();

  // ── Local UI State ──
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [saveModalOpen, setSaveModalOpen] = useState(false);
  const [applyTargetId, setApplyTargetId] = useState<string | null>(null);

  // Reset selection when corridor changes so stale cross-corridor IDs don't cause failed preview queries
  useEffect(() => {
    setSelectedId(null);
    setApplyTargetId(null);
  }, [safeMode]);

  // Default to active collection when nothing selected — guard against stale corridor data
  const effectiveSelectedId =
    selectedId ?? (corridor.isFetching && !corridor.data ? null : corridor.data?.active_collection_id ?? null);

  // ── Handlers ──
  const handleApply = useCallback(
    (collectionId: string, _name: string) => {
      if (!gameId) return;
      setApplyTargetId(collectionId);
    },
    [gameId],
  );

  const handleDelete = useCallback(
    (collectionId: string) => {
      if (!gameId) return;
      deleteMutation.mutate({ gameId, id: collectionId });
      if (selectedId === collectionId) setSelectedId(null);
    },
    [gameId, deleteMutation, selectedId],
  );

  const handleRename = useCallback(
    (collectionId: string, newName: string) => {
      if (!gameId) return;
      updateMutation.mutate({ gameId, id: collectionId, name: newName });
    },
    [gameId, updateMutation],
  );

  const handleUndo = useCallback(() => {
    if (!gameId) return;
    undoMutation.mutate({ gameId });
  }, [gameId, undoMutation]);

  const handleCorridorTabSwitch = useCallback(
    async (targetSafeMode: boolean) => {
      if (safeMode === targetSafeMode) return;
      setSelectedId(null);
      await toggleSafeMode();
    },
    [safeMode, toggleSafeMode],
  );

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
          {corridor.data?.undo_collection_id && (
            <button
              className="btn btn-warning btn-outline btn-sm"
              onClick={handleUndo}
              disabled={undoMutation.isPending}
              title={t('collections:page.actions.undo_title')}
            >
              <Undo2 size={14} />
              {t('collections:page.actions.undo')}
            </button>
          )}
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
                collections={collections.data ?? []}
                selectedId={effectiveSelectedId}
                isLoading={collections.isLoading}
                safeMode={safeMode}
                onSelect={setSelectedId}
                onApply={handleApply}
                onDelete={handleDelete}
                onRename={handleRename}
                onSave={() => setSaveModalOpen(true)}
                isApplying={!!applyTargetId}
                isDeleting={deleteMutation.isPending}
              />
            </div>
          </div>
        </div>

        {/* RIGHT: Preview Panel */}
        <div className="lg:col-span-4 flex flex-col min-h-0 bg-base-200/30 rounded-2xl border border-base-content/5 overflow-hidden shadow-lg">
          <CollectionPreviewPanel collectionId={effectiveSelectedId} gameId={gameId} />
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

      {saveModalOpen && <SaveCollectionModal onClose={() => setSaveModalOpen(false)} />}
      {applyTargetId && (
        <ApplyCollectionModal collectionId={applyTargetId} onClose={() => setApplyTargetId(null)} />
      )}
    </div>
  );
}
