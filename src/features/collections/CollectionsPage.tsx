import { useState } from 'react';
import { Layers, Trash2, Edit2, Check, X, Save, Loader2 } from 'lucide-react';
import { useActiveGame } from '../../hooks/useActiveGame';
import { useAppStore } from '../../stores/useAppStore';
import {
  useApplyCollection,
  useCollections,
  useDeleteCollection,
  useUpdateCollection,
  useActiveModsPreview,
} from './hooks/useCollections';
import type { Collection } from '../../types/collection';
import CollectionWorkspace from './components/CollectionWorkspace';
import ApplyCollectionModal from './components/ApplyCollectionModal';
import SaveCollectionModal from './components/SaveCollectionModal';

export default function CollectionsPage() {
  const { activeGame } = useActiveGame();
  const { safeMode, setSafeMode } = useAppStore();

  const [isSaveModalOpen, setIsSaveModalOpen] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editName, setEditName] = useState('');
  const [selectedCollectionId, setSelectedCollectionId] = useState<string | null>(null);
  const [confirmApply, setConfirmApply] = useState<{
    id: string;
    name: string;
    count: number;
  } | null>(null);

  const collectionsQuery = useCollections(activeGame?.id ?? null);
  const activeModsQuery = useActiveModsPreview(activeGame?.id ?? null, safeMode);
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

  const handleApply = async (collection: Collection) => {
    setConfirmApply({ id: collection.id, name: collection.name, count: collection.member_count });
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

  // Synthesize "Unsaved" collection to guarantee it's always accessible and reflects live state
  const fmtUnsavedName = () => {
    const now = new Date();
    const pad = (n: number) => String(n).padStart(2, '0');
    return `Unsaved ${now.getFullYear()}${pad(now.getMonth() + 1)}${pad(now.getDate())}${pad(now.getHours())}${pad(now.getMinutes())}`;
  };

  let filteredList = collectionsQuery.data?.filter((c) => c.is_safe_context === safeMode) ?? [];
  if (activeGame && collectionsQuery.isSuccess) {
    const activeMods = activeModsQuery.data?.length ?? 0;
    if (!filteredList.some((c) => c.is_last_unsaved) && activeMods > 0) {
      filteredList = [
        {
          id: 'virtual-unsaved',
          name: fmtUnsavedName(),
          game_id: activeGame.id,
          is_safe_context: safeMode,
          member_count: activeMods,
          is_last_unsaved: true,
        },
        ...filteredList,
      ];
    } else if (filteredList.some((c) => c.is_last_unsaved)) {
      filteredList = filteredList.map((c) =>
        c.is_last_unsaved ? { ...c, name: fmtUnsavedName(), member_count: activeMods } : c,
      );
    }
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
              setSafeMode(true);
              setSelectedCollectionId(null);
            }}
          >
            SAFE
          </button>
          <button
            className={`tab tab-sm flex-1 transition-colors ${!safeMode ? 'tab-active bg-error/20 text-error rounded-md! font-medium shadow-sm' : 'text-base-content/60 hover:text-base-content'}`}
            onClick={() => {
              setSafeMode(false);
              setSelectedCollectionId(null);
            }}
          >
            UNSAFE
          </button>
        </div>
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
                  if (filteredList.length === 0) {
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
                        {filteredList.map((collection) => (
                          <tr
                            key={collection.id}
                            onClick={() => setSelectedCollectionId(collection.id)}
                            className={`hover border-white/5 transition-colors group cursor-pointer ${
                              selectedCollectionId === collection.id
                                ? 'bg-primary/10 border-l-2 border-l-primary'
                                : ''
                            }`}
                          >
                            <td className="pl-4">
                              {editingId === collection.id ? (
                                <div className="flex items-center gap-2">
                                  <input
                                    type="text"
                                    className="input input-sm input-bordered w-full max-w-30"
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
                                    className="btn btn-xs btn-square btn-success text-white shrink-0"
                                    onClick={(e) => {
                                      e.stopPropagation();
                                      saveEdit(collection);
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
                                    {collection.name}
                                  </span>
                                  {collection.is_last_unsaved && (
                                    <span className="badge badge-sm badge-warning opacity-90 text-[10px] py-0 h-4 uppercase font-bold tracking-wider shrink-0">
                                      Last Unsaved
                                    </span>
                                  )}
                                  {!collection.is_last_unsaved && (
                                    <button
                                      className="btn btn-xs btn-square btn-ghost opacity-0 group-hover:opacity-100 transition-opacity text-base-content/40 hover:text-white shrink-0"
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
                              <span className="badge badge-sm badge-ghost opacity-70 shrink-0">
                                {collection.member_count} mods
                              </span>
                            </td>

                            <td className="text-right pr-4">
                              <div className="flex items-center justify-end gap-2">
                                <button
                                  className={`btn btn-sm ${collection.is_last_unsaved ? 'btn-secondary' : 'btn-primary'}`}
                                  onClick={(e) => {
                                    e.stopPropagation();
                                    if (collection.is_last_unsaved) {
                                      setIsSaveModalOpen(true);
                                    } else {
                                      handleApply(collection);
                                    }
                                  }}
                                  disabled={
                                    collection.is_last_unsaved
                                      ? false // Never disable Save on the Unsaved preset!
                                      : applyMutation.isPending || collection.member_count === 0
                                  }
                                >
                                  {collection.is_last_unsaved ? (
                                    <>
                                      <Save size={14} className="mr-1" />
                                      Save
                                    </>
                                  ) : (
                                    'Apply'
                                  )}
                                </button>
                                {!collection.is_last_unsaved && (
                                  <button
                                    className="btn btn-sm btn-square btn-ghost text-error/70 hover:text-error hover:bg-error/10 shrink-0"
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
                  );
                })()
              )}
            </div>
          </div>
        </div>

        {/* RIGHT COLUMN: Collection Preview Sidebar */}
        <div className="lg:col-span-4 flex flex-col min-h-0 bg-base-200/30 rounded-2xl border border-white/5 overflow-hidden shadow-lg">
          {selectedCollectionId && filteredList.find((c) => c.id === selectedCollectionId) ? (
            <CollectionWorkspace
              collection={filteredList.find((c) => c.id === selectedCollectionId)!}
              onApply={(collection) => {
                if (collection.is_last_unsaved) {
                  setIsSaveModalOpen(true);
                } else {
                  setConfirmApply({
                    id: collection.id,
                    name: collection.name,
                    count: collection.member_count,
                  });
                }
              }}
              isApplying={applyMutation.isPending}
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
      {isSaveModalOpen && <SaveCollectionModal onClose={() => setIsSaveModalOpen(false)} />}

      {/* Apply Confirmation Modal */}
      {confirmApply && (
        <ApplyCollectionModal
          collectionId={confirmApply.id}
          collectionName={confirmApply.name}
          memberCount={confirmApply.count}
          onClose={() => setConfirmApply(null)}
        />
      )}
    </div>
  );
}
