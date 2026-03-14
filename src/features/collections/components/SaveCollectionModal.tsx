import { useState, useMemo } from 'react';
import { X, Save, Loader2, Package, FolderTree, ShieldAlert } from 'lucide-react';
import { useActiveGame } from '../../../hooks/useActiveGame';
import { useAppStore } from '../../../stores/useAppStore';
import { toast, useToastStore } from '../../../stores/useToastStore';
import { scanService } from '../../../lib/services/scanService';
import { useActiveModsPreview, useSaveCurrentAsCollection } from '../hooks/useCollections';
import { useQueryClient } from '@tanstack/react-query';

interface SaveCollectionModalProps {
  onClose: () => void;
}

export default function SaveCollectionModal({ onClose }: SaveCollectionModalProps) {
  const { activeGame } = useActiveGame();
  const { safeMode } = useAppStore();
  const queryClient = useQueryClient();

  const [name, setName] = useState(() => {
    const now = new Date();
    const yyyy = now.getFullYear();
    const mm = String(now.getMonth() + 1).padStart(2, '0');
    const dd = String(now.getDate()).padStart(2, '0');
    const hh = String(now.getHours()).padStart(2, '0');
    const min = String(now.getMinutes()).padStart(2, '0');
    return `Unsaved ${yyyy}${mm}${dd}${hh}${min}`;
  });
  const [isSyncing, setIsSyncing] = useState(false);
  const [isDisablingAll, setIsDisablingAll] = useState(false);

  const saveMutation = useSaveCurrentAsCollection();
  const activeModsQuery = useActiveModsPreview(activeGame?.id ?? null, safeMode);
  const activeModsData = activeModsQuery.data;

  const groupedActiveMods = useMemo(() => {
    if (!activeModsData) return {};
    return activeModsData.reduce(
      (acc, mod) => {
        const group = mod.object_name || 'Uncategorized';
        if (!acc[group]) acc[group] = [];
        acc[group].push(mod);
        return acc;
      },
      {} as Record<string, typeof activeModsData>,
    );
  }, [activeModsData]);

  const activeModCount = activeModsQuery.data?.length ?? 0;

  const handleSaveCurrent = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim() || !activeGame) return;

    try {
      setIsSyncing(true);
      await scanService.syncDatabase(
        activeGame.id,
        activeGame.name,
        activeGame.game_type,
        activeGame.mod_path,
      );
      setIsSyncing(false);

      await queryClient.invalidateQueries({ queryKey: ['active-mods-preview'] });

      await saveMutation.mutateAsync({
        name: name.trim(),
        game_id: activeGame.id,
        is_safe_context: safeMode,
      });
      toast.success(`Collection "${name.trim()}" saved.`);
      onClose();
    } catch (err) {
      setIsSyncing(false);
      toast.error(`Sync failed: ${String(err)}`);
    }
  };

  const handleDisableAll = async () => {
    if (!activeGame || !activeModsData || activeModsData.length === 0) return;
    setIsDisablingAll(true);
    const toastId = toast.info('Disabling all mods...', 0);
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const paths = activeModsData.map((m) => m.folder_path);

      await invoke('bulk_toggle_mods', {
        paths,
        enable: false,
      });

      queryClient.invalidateQueries({ queryKey: ['active-mods-preview'] });
      queryClient.invalidateQueries({ queryKey: ['objects'] });
      queryClient.invalidateQueries({ queryKey: ['mod-folders'] });

      useToastStore.getState().removeToast(toastId);
      toast.success(`Cleared loadout.`);
    } catch (err) {
      useToastStore.getState().removeToast(toastId);
      toast.error(String(err));
    } finally {
      setIsDisablingAll(false);
    }
  };

  return (
    <div className="fixed inset-0 z-100 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4">
      <div className="card bg-base-200 border border-white/10 shadow-2xl w-full max-w-md my-auto animate-in fade-in zoom-in-95 duration-200">
        <div className="card-body p-6">
          <div className="flex items-center justify-between mb-2">
            <h2 className="card-title text-xl flex gap-2 items-center">
              <Save size={20} className="text-secondary" /> Save Current State
            </h2>
            <button
              className="btn btn-sm btn-circle btn-ghost"
              onClick={onClose}
              disabled={isSyncing || isDisablingAll || saveMutation.isPending}
            >
              <X size={16} />
            </button>
          </div>

          <p className="text-sm text-base-content/60 mb-6">
            Snapshots all currently enabled mods into a new {safeMode ? 'Safe' : 'Unsafe'}{' '}
            collection.
          </p>

          <form onSubmit={handleSaveCurrent} className="space-y-5">
            <div className="form-control">
              <label className="label py-1">
                <span className="label-text font-medium text-base-content/80">Collection Name</span>
              </label>
              <input
                className="input input-bordered focus:border-secondary bg-base-300 w-full"
                placeholder="e.g. Abyss Run 1"
                value={name}
                onChange={(e) => setName(e.target.value)}
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
                <p className="text-[10px] text-base-content/50 leading-tight">
                  To save a {safeMode ? 'Unsafe' : 'Safe'} collection, close this and switch tabs.
                </p>
              </div>
            </div>

            <div className="border border-white/5 rounded-xl bg-base-300/20 overflow-hidden">
              <div className="flex items-center justify-between bg-base-300/50 p-3 border-b border-white/5">
                <h3 className="text-xs font-bold text-base-content/70 uppercase tracking-wider flex items-center gap-1.5">
                  <Package size={14} />
                  Enabled Mods ({activeModCount})
                </h3>
                {activeModCount > 0 && (
                  <button
                    type="button"
                    onClick={handleDisableAll}
                    disabled={isDisablingAll || isSyncing || saveMutation.isPending}
                    className="btn btn-xs btn-outline btn-error opacity-80 hover:opacity-100"
                  >
                    {isDisablingAll ? (
                      <Loader2 size={12} className="animate-spin" />
                    ) : (
                      'Disable All'
                    )}
                  </button>
                )}
              </div>

              <div className="p-2">
                {activeModsQuery.isLoading ? (
                  <div className="flex justify-center py-6 text-base-content/40">
                    <Loader2 size={20} className="animate-spin" />
                  </div>
                ) : activeModCount === 0 ? (
                  <p className="text-xs text-center py-6 text-base-content/40 italic">
                    No enabled mods to save.
                  </p>
                ) : (
                  <div className="space-y-3 max-h-48 overflow-y-auto custom-scrollbar pr-2 pl-1 py-1">
                    {Object.keys(groupedActiveMods)
                      .sort()
                      .map((groupName) => (
                        <div key={groupName}>
                          <div className="text-[11px] font-bold text-secondary/80 uppercase tracking-wider mb-1">
                            {groupName}{' '}
                            <span className="text-base-content/30 ml-1">
                              ({groupedActiveMods[groupName].length})
                            </span>
                          </div>
                          <ul className="space-y-1">
                            {groupedActiveMods[groupName].map((mod) => (
                              <li
                                key={mod.id}
                                className="text-xs text-base-content/80 flex items-center gap-2 pl-2 p-1 rounded hover:bg-white/5 transition-colors"
                                title={mod.folder_path}
                              >
                                {mod.id.startsWith('nested_') && (
                                  <span title="Nested Mod" className="flex shrink-0">
                                    <FolderTree size={12} className="text-info/70" />
                                  </span>
                                )}
                                {!mod.is_safe && (
                                  <span title="Unsafe / Non-Safe Mod" className="flex shrink-0">
                                    <ShieldAlert size={12} className="text-warning" />
                                  </span>
                                )}
                                <span className="truncate">{mod.actual_name}</span>
                              </li>
                            ))}
                          </ul>
                        </div>
                      ))}
                  </div>
                )}
              </div>
            </div>

            <div className="pt-2">
              <button
                type="submit"
                disabled={
                  !name.trim() ||
                  saveMutation.isPending ||
                  isSyncing ||
                  activeModCount === 0 ||
                  isDisablingAll
                }
                className="btn btn-secondary w-full"
              >
                {isSyncing ? (
                  <>
                    <Loader2 size={18} className="animate-spin" /> Syncing State...
                  </>
                ) : saveMutation.isPending ? (
                  <>
                    <Loader2 size={18} className="animate-spin" /> Saving...
                  </>
                ) : (
                  `Save Collection`
                )}
              </button>
            </div>
          </form>
        </div>
      </div>
    </div>
  );
}
