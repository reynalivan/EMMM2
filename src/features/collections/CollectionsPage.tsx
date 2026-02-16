import { useMemo, useState } from 'react';
import { Download, Layers, Trash2, Upload, WandSparkles } from 'lucide-react';
import { useActiveGame } from '../../hooks/useActiveGame';
import { useAppStore } from '../../stores/useAppStore';
import { toast } from '../../stores/useToastStore';
import type { ExportCollectionPayload } from '../../types/collection';
import {
  useApplyCollection,
  useCollections,
  useCreateCollection,
  useDeleteCollection,
  useExportCollection,
  useImportCollection,
  useUndoCollectionApply,
} from './hooks/useCollections';

export default function CollectionsPage() {
  const { activeGame } = useActiveGame();
  const safeMode = useAppStore((state) => state.safeMode);
  const gridSelection = useAppStore((state) => state.gridSelection);

  const [name, setName] = useState('');
  const [isSafeContext, setIsSafeContext] = useState(true);
  const [exportText, setExportText] = useState('');
  const [importText, setImportText] = useState('');

  const collectionsQuery = useCollections(activeGame?.id ?? null);
  const createMutation = useCreateCollection();
  const deleteMutation = useDeleteCollection();
  const applyMutation = useApplyCollection();
  const undoMutation = useUndoCollectionApply();
  const exportMutation = useExportCollection();
  const importMutation = useImportCollection();

  const selectedMods = useMemo(() => Array.from(gridSelection), [gridSelection]);

  const canCreate = useMemo(() => {
    return Boolean(activeGame?.id && name.trim().length > 1 && selectedMods.length > 0);
  }, [activeGame?.id, name, selectedMods.length]);

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

  const handleCreate = async () => {
    if (!canCreate) return;
    await createMutation.mutateAsync({
      name: name.trim(),
      game_id: activeGame.id,
      is_safe_context: isSafeContext,
      mod_ids: selectedMods,
    });
    setName('');
  };

  const handleApply = async (collectionId: string) => {
    const result = await applyMutation.mutateAsync({ collectionId, gameId: activeGame.id });
    if (result.warnings.length > 0) {
      toast.warning(result.warnings.join(', '));
    }
  };

  const handleUndo = async () => {
    await undoMutation.mutateAsync(activeGame.id);
  };

  const handleExport = async (collectionId: string) => {
    const payload = await exportMutation.mutateAsync({ collectionId, gameId: activeGame.id });
    setExportText(JSON.stringify(payload, null, 2));
  };

  const handleImport = async () => {
    if (!importText.trim()) {
      toast.warning('Paste export JSON first');
      return;
    }

    let payload: ExportCollectionPayload;
    try {
      payload = JSON.parse(importText) as ExportCollectionPayload;
    } catch {
      toast.error('Invalid JSON');
      return;
    }

    await importMutation.mutateAsync({ gameId: activeGame.id, payload });
  };

  return (
    <div className="h-full overflow-y-auto p-4 md:p-6 bg-base-100">
      <div className="max-w-6xl mx-auto space-y-6">
        <div className="flex items-center justify-between gap-4">
          <div>
            <h1 className="text-2xl font-bold tracking-tight flex items-center gap-2">
              <Layers size={22} className="text-primary" />
              Collections
            </h1>
            <p className="text-sm text-base-content/70 mt-1">
              Build presets from selected mods and apply them atomically.
            </p>
          </div>
          <button
            onClick={handleUndo}
            className="btn btn-outline btn-warning"
            disabled={undoMutation.isPending}
          >
            <WandSparkles size={16} /> Undo Last Apply
          </button>
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
          <div className="card bg-base-200 border border-base-300 shadow-sm lg:col-span-1">
            <div className="card-body">
              <h2 className="card-title text-base">Create Collection</h2>
              <label className="form-control">
                <span className="label-text mb-1">Name</span>
                <input
                  className="input input-bordered"
                  placeholder="Abyss Team"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                />
              </label>

              <label className="label cursor-pointer justify-start gap-3">
                <input
                  type="checkbox"
                  className="checkbox checkbox-sm"
                  checked={isSafeContext}
                  onChange={(e) => setIsSafeContext(e.target.checked)}
                />
                <span className="label-text">Safe Context</span>
              </label>

              <div className="text-xs text-base-content/60">
                Selected mods: <span className="font-semibold">{selectedMods.length}</span>
                {safeMode && !isSafeContext && (
                  <div className="text-warning mt-1">
                    Safe Mode is ON. NSFW context collections may be blocked on apply.
                  </div>
                )}
              </div>

              <button
                onClick={handleCreate}
                disabled={!canCreate || createMutation.isPending}
                className="btn btn-primary"
              >
                Create
              </button>
            </div>
          </div>

          <div className="card bg-base-200 border border-base-300 shadow-sm lg:col-span-2">
            <div className="card-body">
              <h2 className="card-title text-base">Collections for {activeGame.name}</h2>
              {collectionsQuery.isLoading && <p className="text-sm">Loading collections...</p>}
              {!collectionsQuery.isLoading && (collectionsQuery.data?.length ?? 0) === 0 && (
                <p className="text-sm text-base-content/70">No collections yet.</p>
              )}

              <div className="space-y-2">
                {collectionsQuery.data?.map((collection) => (
                  <div
                    key={collection.id}
                    className="flex items-center justify-between gap-2 p-3 rounded-lg bg-base-100 border border-base-300"
                  >
                    <div>
                      <div className="font-medium">{collection.name}</div>
                      <div className="text-xs text-base-content/60">
                        <span
                          className={`badge badge-xs ${collection.is_safe_context ? 'badge-success' : 'badge-error'}`}
                        >
                          {collection.is_safe_context ? 'SAFE' : 'NSFW'}
                        </span>
                      </div>
                    </div>
                    <div className="flex items-center gap-2">
                      <button
                        className="btn btn-xs btn-primary"
                        onClick={() => handleApply(collection.id)}
                        disabled={applyMutation.isPending}
                        aria-label={`Apply ${collection.name}`}
                      >
                        Apply
                      </button>
                      <button
                        className="btn btn-xs btn-ghost"
                        onClick={() => handleExport(collection.id)}
                        disabled={exportMutation.isPending}
                        aria-label={`Export ${collection.name}`}
                      >
                        <Download size={14} />
                      </button>
                      <button
                        className="btn btn-xs btn-ghost text-error"
                        onClick={() =>
                          deleteMutation.mutate({ id: collection.id, gameId: activeGame.id })
                        }
                        disabled={deleteMutation.isPending}
                        aria-label={`Delete ${collection.name}`}
                      >
                        <Trash2 size={14} />
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
          <div className="card bg-base-200 border border-base-300 shadow-sm">
            <div className="card-body">
              <h2 className="card-title text-base">
                <Download size={16} /> Export Preview
              </h2>
              <textarea
                className="textarea textarea-bordered h-56 font-mono text-xs"
                value={exportText}
                onChange={(e) => setExportText(e.target.value)}
                placeholder="Exported JSON will appear here"
              />
            </div>
          </div>

          <div className="card bg-base-200 border border-base-300 shadow-sm">
            <div className="card-body">
              <h2 className="card-title text-base">
                <Upload size={16} /> Import JSON
              </h2>
              <textarea
                className="textarea textarea-bordered h-56 font-mono text-xs"
                value={importText}
                onChange={(e) => setImportText(e.target.value)}
                placeholder="Paste exported JSON"
              />
              <button
                onClick={handleImport}
                disabled={importMutation.isPending}
                className="btn btn-secondary"
              >
                Import
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
