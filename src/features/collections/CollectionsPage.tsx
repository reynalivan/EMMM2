import { useState } from 'react';
import { Layers, Trash2, Edit2, Check, X, Save, Loader2, AlertTriangle } from 'lucide-react';
import { useActiveGame } from '../../hooks/useActiveGame';
import { useAppStore } from '../../stores/useAppStore';
import { toast } from '../../stores/useToastStore';
import {
  useApplyCollection,
  useCollections,
  useSaveCurrentAsCollection,
  useDeleteCollection,
  useUpdateCollection,
} from './hooks/useCollections';
import type { Collection } from '../../types/collection';
import CollectionSidebar from './CollectionSidebar';

export default function CollectionsPage() {
  const { activeGame } = useActiveGame();
  const { safeMode, setSafeMode } = useAppStore();

  const [name, setName] = useState('');
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editName, setEditName] = useState('');
  const [selectedCollectionId, setSelectedCollectionId] = useState<string | null>(null);
  const [confirmApply, setConfirmApply] = useState<{
    id: string;
    name: string;
    count: number;
  } | null>(null);

  const collectionsQuery = useCollections(activeGame?.id ?? null);
  const saveMutation = useSaveCurrentAsCollection();
  const updateMutation = useUpdateCollection();
  const deleteMutation = useDeleteCollection();
  const applyMutation = useApplyCollection();

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

  const handleSaveCurrent = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return;
    await saveMutation.mutateAsync({
      name: name.trim(),
      game_id: activeGame.id,
      is_safe_context: safeMode,
    });
    setName('');
  };

  const handleApply = async (collection: Collection) => {
    setConfirmApply({ id: collection.id, name: collection.name, count: collection.member_count });
  };

  const confirmApplyAction = async () => {
    if (!confirmApply) return;
    const result = await applyMutation.mutateAsync({
      collectionId: confirmApply.id,
      gameId: activeGame.id,
    });
    if (result.warnings.length > 0) {
      toast.warning(result.warnings.join(', '));
    }
    setConfirmApply(null);
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

  return (
    <div className="h-full overflow-y-auto p-4 md:p-8 bg-base-100/50">
      <div className="max-w-5xl mx-auto space-y-8">
        <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
          <div>
            <h1 className="text-3xl font-bold tracking-tight flex items-center gap-3">
              <Layers size={28} className="text-primary" />
              Collections
            </h1>
            <p className="text-base-content/60 mt-2 max-w-2xl">
              Save your currently enabled mods and variants as a named collection. Apply collections
              atomically to instantly switch your entire game loadout.
            </p>
          </div>
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
          {/* Create new collection panel */}
          <div className="lg:col-span-3">
            <div className="card bg-base-200/50 border border-white/5 shadow-xl backdrop-blur-md sticky top-4">
              <div className="card-body">
                <h2 className="card-title text-lg flex gap-2 items-center mb-2">
                  <Save size={18} className="text-secondary" /> Save Current State
                </h2>
                <p className="text-xs text-base-content/60 mb-4">
                  Snapshots all currently enabled mods across all objects into a new collection.
                </p>
                <form onSubmit={handleSaveCurrent} className="space-y-4">
                  <div className="form-control">
                    <label className="label py-1">
                      <span className="label-text text-xs opacity-70">Collection Name</span>
                    </label>
                    <input
                      className="input input-sm input-bordered focus:border-secondary bg-base-300 w-full"
                      placeholder="e.g. Abyss Run 1"
                      value={name}
                      onChange={(e) => setName(e.target.value)}
                      required
                    />
                  </div>

                  <div className="form-control bg-base-300/30 p-3 rounded-lg border border-white/5 space-y-1">
                    <p className="text-xs text-base-content/70">
                      Context:{' '}
                      <span className={`font-semibold ${safeMode ? 'text-success' : 'text-error'}`}>
                        {safeMode ? 'SAFE' : 'NSFW'}
                      </span>
                    </p>
                    <p className="text-[10px] text-base-content/50 leading-tight">
                      To save a {safeMode ? 'NSFW' : 'Safe'} collection, switch the active tab or
                      toggle Safe Mode in the topbar.
                    </p>
                  </div>

                  <button
                    type="submit"
                    disabled={!name.trim() || saveMutation.isPending}
                    className="btn btn-secondary btn-sm w-full mt-2"
                  >
                    {saveMutation.isPending ? (
                      <Loader2 size={16} className="animate-spin" />
                    ) : (
                      'Save Collection'
                    )}
                  </button>
                </form>
              </div>
            </div>
          </div>

          {/* Collections List */}
          <div className={selectedCollectionId ? 'lg:col-span-5' : 'lg:col-span-9'}>
            <div className="card bg-base-200/30 border border-white/5 shadow-lg min-h-[500px] flex flex-col transition-all duration-300">
              <div className="p-0 border-b border-white/5 bg-base-200/50 rounded-t-2xl">
                <div className="tabs tabs-boxed bg-transparent p-2 gap-2 w-full max-w-sm mx-auto my-2">
                  <button
                    className={`tab tab-sm flex-1 ${safeMode ? 'tab-active bg-success/20 text-success rounded-md! font-medium' : 'text-base-content/60'}`}
                    onClick={() => setSafeMode(true)}
                  >
                    SAFE Collections
                  </button>
                  <button
                    className={`tab tab-sm flex-1 ${!safeMode ? 'tab-active bg-error/20 text-error rounded-md! font-medium' : 'text-base-content/60'}`}
                    onClick={() => setSafeMode(false)}
                  >
                    NSFW Collections
                  </button>
                </div>
              </div>

              <div className="card-body p-0 flex-1">
                {collectionsQuery.isLoading ? (
                  <div className="flex justify-center py-12 text-base-content/50">
                    <Loader2 size={24} className="animate-spin" />
                  </div>
                ) : (
                  (() => {
                    const filteredList =
                      collectionsQuery.data?.filter((c) => c.is_safe_context === safeMode) ?? [];
                    if (filteredList.length === 0) {
                      return (
                        <div className="flex flex-col items-center justify-center py-16 px-4 text-center h-full">
                          <div className="w-16 h-16 rounded-full bg-base-300 flex items-center justify-center mb-4 text-base-content/30 mt-8">
                            <Layers size={32} />
                          </div>
                          <h3 className="text-lg font-medium opacity-80 mb-2">
                            No {safeMode ? 'Safe' : 'NSFW'} collections found
                          </h3>
                          <p className="text-sm opacity-50 max-w-sm">
                            Create your first {safeMode ? 'safe' : 'NSFW'} collection by entering a
                            name on the left and saving your current mod state.
                          </p>
                        </div>
                      );
                    }

                    return (
                      <div className="overflow-x-auto">
                        <table className="table table-auto w-full">
                          <thead>
                            <tr className="border-white/5 text-base-content/50">
                              <th className="w-1/2">Name</th>
                              <th>Mods</th>
                              <th className="text-right pr-6">Actions</th>
                            </tr>
                          </thead>
                          <tbody>
                            {filteredList.map((collection) => (
                              <tr
                                key={collection.id}
                                onClick={() => setSelectedCollectionId(collection.id)}
                                className={`hover border-white/5 transition-colors group cursor-pointer ${
                                  selectedCollectionId === collection.id ? 'bg-white/5' : ''
                                }`}
                              >
                                <td>
                                  {editingId === collection.id ? (
                                    <div className="flex items-center gap-2">
                                      <input
                                        type="text"
                                        className="input input-sm input-bordered w-full max-w-xs"
                                        value={editName}
                                        onChange={(e) => setEditName(e.target.value)}
                                        onClick={(e) => e.stopPropagation()}
                                        autoFocus
                                        onKeyDown={(e) => {
                                          if (e.key === 'Enter') saveEdit(collection);
                                          if (e.key === 'Escape') cancelEdit();
                                        }}
                                      />
                                      <button
                                        className="btn btn-xs btn-square btn-success text-white"
                                        onClick={(e) => {
                                          e.stopPropagation();
                                          saveEdit(collection);
                                        }}
                                      >
                                        <Check size={14} />
                                      </button>
                                      <button
                                        className="btn btn-xs btn-square btn-ghost"
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
                                      {collection.name}
                                      {collection.is_last_unsaved && (
                                        <span className="badge badge-sm badge-warning opacity-90 text-[10px] py-0 h-4 uppercase font-bold tracking-wider">
                                          Last Unsaved
                                        </span>
                                      )}
                                      {!collection.is_last_unsaved && (
                                        <button
                                          className="btn btn-xs btn-square btn-ghost opacity-0 group-hover:opacity-100 transition-opacity text-base-content/40 hover:text-white"
                                          onClick={(e) => {
                                            e.stopPropagation();
                                            startEdit(collection);
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
                                  <span className="badge badge-sm badge-ghost opacity-70">
                                    {collection.member_count} mods
                                  </span>
                                </td>

                                <td className="text-right pr-4">
                                  <div className="flex items-center justify-end gap-2">
                                    <button
                                      className="btn btn-sm btn-primary"
                                      onClick={(e) => {
                                        e.stopPropagation();
                                        handleApply(collection);
                                      }}
                                      disabled={
                                        applyMutation.isPending || collection.member_count === 0
                                      }
                                    >
                                      Apply
                                    </button>
                                    {!collection.is_last_unsaved && (
                                      <button
                                        className="btn btn-sm btn-square btn-ghost text-error/70 hover:text-error hover:bg-error/10"
                                        onClick={(e) => {
                                          e.stopPropagation();
                                          deleteMutation.mutate({
                                            id: collection.id,
                                            gameId: activeGame.id,
                                          });
                                          if (selectedCollectionId === collection.id) {
                                            setSelectedCollectionId(null);
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
                            ))}
                          </tbody>
                        </table>
                      </div>
                    );
                  })()
                )}
              </div>
            </div>
          </div>

          {/* Collection Sidebar Panel */}
          {selectedCollectionId && (
            <CollectionSidebar
              collectionId={selectedCollectionId}
              onClose={() => setSelectedCollectionId(null)}
            />
          )}
        </div>
      </div>
      {/* Apply Confirmation Modal */}
      {confirmApply && (
        <dialog className="modal modal-open z-50">
          <div className="modal-box bg-base-200 border border-white/10 shadow-2xl max-w-sm">
            <h3 className="font-bold text-lg mb-3 flex items-center gap-2">
              <AlertTriangle size={20} className="text-warning" />
              Confirm Apply
            </h3>
            <p className="text-sm text-base-content/70 mb-2">
              Apply <strong className="text-white">"{confirmApply.name}"</strong>?
            </p>
            <p className="text-xs text-base-content/50 mb-5">
              This will enable {confirmApply.count} mod(s) and disable conflicting mods in the same
              object categories. A snapshot of your current state will be saved automatically.
            </p>
            <div className="modal-action">
              <button
                className="btn btn-ghost btn-sm"
                onClick={() => setConfirmApply(null)}
                disabled={applyMutation.isPending}
              >
                Cancel
              </button>
              <button
                data-testid="modal-apply-btn"
                className="btn btn-primary btn-sm min-w-[80px]"
                onClick={confirmApplyAction}
                disabled={applyMutation.isPending}
              >
                {applyMutation.isPending ? <Loader2 size={14} className="animate-spin" /> : 'Apply'}
              </button>
            </div>
          </div>
          <form method="dialog" className="modal-backdrop">
            <button onClick={() => setConfirmApply(null)}>close</button>
          </form>
        </dialog>
      )}
    </div>
  );
}
