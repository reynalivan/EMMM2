import { ShieldCheck, ShieldAlert, Save, Layers, Loader2 } from 'lucide-react';
import { useAppStore } from '../../../stores/useAppStore';
import {
  useCollections,
  useApplyCollection,
  useSaveCurrentAsCollection,
} from '../../../features/collections/hooks/useCollections';
import { useState } from 'react';
import { createPortal } from 'react-dom';

export default function ContextControls() {
  const {
    safeMode,
    setSafeMode,
    activeCollectionId,
    setActiveCollectionId,
    activeGameId,
    setWorkspaceView,
  } = useAppStore();
  const { data: collections = [], isLoading } = useCollections(activeGameId);
  const applyMutation = useApplyCollection();
  const saveMutation = useSaveCurrentAsCollection();

  const [saveModalOpen, setSaveModalOpen] = useState(false);
  const [newCollectionName, setNewCollectionName] = useState('');

  const activeCollection = collections.find((c) => c.id === activeCollectionId);
  const triggerText = activeCollection ? activeCollection.name : 'Collections';

  const handleApply = (id: string) => {
    if (!activeGameId) return;
    setActiveCollectionId(id);
    applyMutation.mutate({ collectionId: id, gameId: activeGameId });
    const elem = document.activeElement as HTMLElement;
    if (elem) elem.blur();
  };

  const handleSaveCurrent = (e: React.FormEvent) => {
    e.preventDefault();
    if (!activeGameId || !newCollectionName.trim()) return;

    saveMutation.mutate(
      {
        name: newCollectionName.trim(),
        game_id: activeGameId,
        is_safe_context: safeMode,
      },
      {
        onSuccess: (res) => {
          setSaveModalOpen(false);
          setNewCollectionName('');
          setActiveCollectionId(res.collection.id);
        },
      },
    );
  };

  return (
    <>
      <div className="hidden lg:flex items-center gap-3 bg-base-100/30 p-1.5 rounded-full border border-white/5 backdrop-blur-md">
        <label
          className={`btn btn-xs btn-circle border-0 ${safeMode ? 'bg-success/20 text-success hover:bg-success/30' : 'bg-error/20 text-error hover:bg-error/30'}`}
          onClick={() => setSafeMode(!safeMode)}
          title="Safe Mode Toggle"
        >
          {safeMode ? <ShieldCheck size={14} /> : <ShieldAlert size={14} />}
        </label>

        <div className="h-4 w-px bg-white/10" />

        <div className="dropdown dropdown-bottom dropdown-end">
          <div
            tabIndex={0}
            role="button"
            className="px-3 py-1 rounded-full text-xs font-medium text-white/70 hover:text-white hover:bg-white/5 transition-all cursor-pointer flex items-center gap-2 max-w-[150px]"
            title={triggerText}
          >
            <span className="truncate">{triggerText}</span>{' '}
            <span className="text-[9px] opacity-30">▼</span>
          </div>
          <ul
            tabIndex={0}
            className="dropdown-content z-50 menu p-2 shadow-2xl bg-base-100/95 backdrop-blur-xl rounded-box w-56 mt-2 border border-white/10"
          >
            <li className="menu-title text-[10px] uppercase opacity-40 px-2 pb-1 tracking-widest flex justify-between items-center">
              <span>Collections</span>
            </li>

            <li>
              <button
                className="hover:bg-primary/20 text-primary text-sm gap-2"
                onClick={() => {
                  setSaveModalOpen(true);
                  const elem = document.activeElement as HTMLElement;
                  if (elem) elem.blur();
                }}
              >
                <Save size={14} />
                Save Current
              </button>
            </li>

            <div className="divider my-1 before:bg-white/5 after:bg-white/5 mx-2"></div>

            {isLoading ? (
              <li className="disabled">
                <span className="text-xs opacity-50 flex gap-2">
                  <Loader2 size={12} className="animate-spin" /> Loading...
                </span>
              </li>
            ) : (
              (() => {
                const userCollections = collections.filter((c) => !c.is_last_unsaved);
                return userCollections.length === 0 ? (
                  <li className="disabled">
                    <span className="text-xs opacity-50 px-2">No collections saved</span>
                  </li>
                ) : (
                  <div className="max-h-[30vh] overflow-y-auto pr-1 custom-scrollbar">
                    {userCollections.map((c) => (
                      <li key={c.id}>
                        <button
                          className={`text-sm justify-between ${activeCollectionId === c.id ? 'bg-white/10 text-white font-medium' : 'hover:bg-white/5'}`}
                          onClick={() => handleApply(c.id)}
                        >
                          <span className="truncate max-w-[130px]">{c.name}</span>
                          <span className="badge badge-xs badge-ghost opacity-50">
                            {c.member_count}
                          </span>
                        </button>
                      </li>
                    ))}
                  </div>
                );
              })()
            )}

            <div className="divider my-1 before:bg-white/5 after:bg-white/5 mx-2"></div>

            <li>
              <button
                className="hover:bg-white/5 text-sm gap-2 text-white/70"
                onClick={() => {
                  setWorkspaceView('collections');
                  const elem = document.activeElement as HTMLElement;
                  if (elem) elem.blur();
                }}
              >
                <Layers size={14} />
                Manage Collections
              </button>
            </li>
          </ul>
        </div>
      </div>

      {saveModalOpen &&
        createPortal(
          <dialog className="modal modal-open z-100">
            <div className="modal-box bg-base-200 border border-white/10 shadow-2xl">
              <h3 className="font-bold text-lg mb-4 flex items-center gap-2">
                <Save size={20} className="text-primary" />
                Save Current Setup
              </h3>
              <p className="text-sm text-base-content/70 mb-5">
                This will create a new collection from your currently enabled mods and variants.
              </p>
              <form onSubmit={handleSaveCurrent}>
                <div className="form-control mb-6">
                  <label className="label">
                    <span className="label-text opacity-70">Collection Name</span>
                  </label>
                  <input
                    type="text"
                    placeholder="e.g. Boss Fight Loadout"
                    className="input input-bordered w-full bg-base-300 focus:border-primary transition-colors"
                    value={newCollectionName}
                    onChange={(e) => setNewCollectionName(e.target.value)}
                    autoFocus
                    required
                  />
                </div>
                <div className="modal-action">
                  <button
                    type="button"
                    className="btn btn-ghost hover:bg-white/5 text-white/70"
                    onClick={() => setSaveModalOpen(false)}
                    disabled={saveMutation.isPending}
                  >
                    Cancel
                  </button>
                  <button
                    type="submit"
                    className="btn btn-primary min-w-[100px]"
                    disabled={!newCollectionName.trim() || saveMutation.isPending}
                  >
                    {saveMutation.isPending ? (
                      <Loader2 size={16} className="animate-spin" />
                    ) : (
                      'Save'
                    )}
                  </button>
                </div>
              </form>
            </div>
            <form method="dialog" className="modal-backdrop">
              <button onClick={() => setSaveModalOpen(false)}>close</button>
            </form>
          </dialog>,
          document.body,
        )}
    </>
  );
}
