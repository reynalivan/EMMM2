/**
 * CollectionsPage — Thin layout component for the collections feature.
 *
 * **658 → ~120 lines** — All state derivation removed.
 * Backend computes: active collection, undo target, dirty state, signatures.
 * Frontend renders the runtime status, collection list, and preview.
 *
 * Replaces: 6 useMemo chains, 3 useEffect syncs, resolveActiveCollection,
 *           findWorkspaceRowByCollectionId.
 */

import { useState, useCallback, useEffect, useMemo } from 'react';
import { History, Layers, RotateCcw, Save, Trash2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useActiveGame } from '../../hooks/useActiveGame';

import {
  useClearLastChanges,
  useCollectionRuntime,
  useCollections,
  useDeleteCollection,
  useRestoreLastChanges,
  useSaveCollectionChanges,
  useUpdateCollection,
} from './hooks';
import { CollectionList } from './components/CollectionList';
import { CollectionPreviewPanel } from './components/CollectionPreviewPanel';
import { SaveCollectionModal } from './components/SaveCollectionModal';
import { ApplyCollectionModal } from './components/ApplyCollectionModal';
import {
  buildCollectionWorkspaceRows,
  CURRENT_RUNTIME_ROW_ID,
  filterCollectionRowsBySafety,
  isCollectionWorkspaceSourceEqual,
  type CollectionListRow,
  type CollectionSaveRequest,
  type CollectionWorkspaceSource,
} from './types';
import { useAppStore } from '../../stores/useAppStore';
import { SafetyFilterControl } from '../../components/ui/SafetyFilterControl';
import { extractMissingModsPayload } from '../../lib/appError';

export default function CollectionsPage() {
  const { t } = useTranslation('collections');
  const { activeGame } = useActiveGame();

  const gameId = activeGame?.id ?? null;
  const safetyFilter = useAppStore((state) => state.safetyFilter);
  const setSafetyFilter = useAppStore((state) => state.setSafetyFilter);

  // ── v2 Queries ──
  const runtime = useCollectionRuntime(gameId);
  const collections = useCollections(gameId);

  // ── v2 Mutations ──
  const deleteMutation = useDeleteCollection();
  const updateMutation = useUpdateCollection();
  const saveChangesMutation = useSaveCollectionChanges();
  const restoreLastChangesMutation = useRestoreLastChanges();
  const clearLastChangesMutation = useClearLastChanges();

  // ── Local UI State ──
  const [selectedSource, setSelectedSource] = useState<CollectionWorkspaceSource | null>(null);
  const [saveModalOpen, setSaveModalOpen] = useState(false);
  const [saveRequest, setSaveRequest] = useState<CollectionSaveRequest | null>(null);
  const [applyTargetId, setApplyTargetId] = useState<string | null>(null);

  // Reset selection when game changes so stale cross-game IDs don't cause failed preview queries.
  useEffect(() => {
    setSelectedSource(null);
    setApplyTargetId(null);
    setSaveRequest(null);
  }, [gameId]);

  const rows = useMemo<CollectionListRow[]>(() => {
    return filterCollectionRowsBySafety(
      buildCollectionWorkspaceRows(
        collections.data ?? [],
        runtime.data,
        t('list.item.current_runtime', 'Current changes'),
      ),
      safetyFilter,
    );
  }, [collections.data, runtime.data, safetyFilter, t]);

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

    const activeCollectionId = runtime.data?.active_collection_id;
    if (activeCollectionId && hasStoredCollection(activeCollectionId)) {
      return { kind: 'stored_collection', collectionId: activeCollectionId };
    }

    if (runtime.data?.is_dirty && hasCurrentRuntime) {
      return { kind: 'current_runtime' };
    }

    const firstStored = rows.find((row) => row.kind === 'stored_collection');
    if (firstStored && firstStored.kind === 'stored_collection') {
      return { kind: 'stored_collection', collectionId: firstStored.collection.id };
    }

    if (hasCurrentRuntime) {
      return { kind: 'current_runtime' };
    }

    return null;
  }, [runtime.data, rows, selectedSource]);

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

  const handleSave = useCallback((request: CollectionSaveRequest) => {
    setSaveRequest(request);
    setSaveModalOpen(true);
  }, []);

  const handleSaveChanges = useCallback(
    async (collectionId: string) => {
      if (!gameId) return;
      try {
        await saveChangesMutation.mutateAsync({
          gameId,
          collectionId,
          confirmRemoveMissing: false,
        });
      } catch (error) {
        const missing = extractMissingModsPayload(error);
        if (!missing) return;
        const confirmed = window.confirm(
          t('last_changes.remove_missing_confirm', {
            count: missing.paths.length,
            defaultValue: `Remove ${missing.paths.length} missing mod(s) from this collection?`,
          }),
        );
        if (confirmed) {
          await saveChangesMutation.mutateAsync({
            gameId,
            collectionId,
            confirmRemoveMissing: true,
          });
        }
      }
    },
    [gameId, saveChangesMutation, t],
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

        <div className="flex items-center gap-2">
          <SafetyFilterControl value={safetyFilter} onChange={setSafetyFilter} />
          <button className="btn btn-secondary btn-sm" onClick={() => setSaveModalOpen(true)}>
            <Save size={14} />
            {t('collections:page.actions.save_current')}
          </button>
        </div>
      </div>

      {runtime.data?.last_changes?.source === 'draft' && (
        <div className="mb-4 flex flex-col gap-3 rounded-xl border border-warning/25 bg-warning/8 p-3 sm:flex-row sm:items-center sm:justify-between">
          <div className="flex min-w-0 items-center gap-3">
            <History className="shrink-0 text-warning" size={20} />
            <div>
              <div className="font-semibold">{t('last_changes.title', 'Last changes')}</div>
              <div className="text-xs text-base-content/60">
                {t(
                  'last_changes.description',
                  'A modified or unsaved runtime was preserved before switching collections.',
                )}
              </div>
            </div>
          </div>
          <div className="flex flex-wrap gap-2">
            {runtime.data.last_changes.collection_id && (
              <button
                className="btn btn-xs btn-ghost"
                onClick={() => {
                  setSaveRequest({
                    mode: 'clone_snapshot',
                    sourceCollectionId: runtime.data?.last_changes?.collection_id ?? null,
                  });
                  setSaveModalOpen(true);
                }}
              >
                <Save size={13} /> {t('last_changes.save_as', 'Save as collection')}
              </button>
            )}
            <button
              className="btn btn-xs btn-primary"
              disabled={restoreLastChangesMutation.isPending}
              onClick={() => gameId && restoreLastChangesMutation.mutate(gameId)}
            >
              <RotateCcw size={13} /> {t('last_changes.restore', 'Restore')}
            </button>
            <button
              className="btn btn-xs btn-ghost text-error"
              disabled={clearLastChangesMutation.isPending}
              onClick={() => gameId && clearLastChangesMutation.mutate(gameId)}
            >
              <Trash2 size={13} /> {t('last_changes.clear', 'Clear')}
            </button>
          </div>
        </div>
      )}

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
                isError={collections.isError}
                error={collections.error}
                onSelect={handleSelect}
                onApply={handleApply}
                onDelete={handleDelete}
                onRename={handleRename}
                onSave={handleSave}
                onSaveChanges={handleSaveChanges}
                activeCollectionId={runtime.data?.active_collection_id}
                runtimeStatus={runtime.data?.runtime_status}
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
            runtimeSnapshot={runtime.data}
          />
        </div>
      </div>

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
        <ApplyCollectionModal collectionId={applyTargetId} onClose={() => setApplyTargetId(null)} />
      )}
    </div>
  );
}
