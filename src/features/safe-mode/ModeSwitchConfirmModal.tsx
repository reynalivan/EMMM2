import { useMemo, useState, useEffect } from 'react';
import { createPortal } from 'react-dom';
import { invoke } from '@tauri-apps/api/core';
import { ShieldCheck, ShieldAlert, Loader2, ArrowRight } from 'lucide-react';
import { useAppStore } from '../../stores/useAppStore';
import { useActiveModsPreview, useCollectionPreview } from '../collections/hooks/useCollections';
import { ModGroupList } from '../collections/components/ModGroupList';
import { groupMods } from '../collections/utils/groupMods';

// Inline type for the backend result
interface ModeSwitchPreviewResult {
  target_coll_id: string | null;
  target_name: string;
}

interface ModeSwitchConfirmModalProps {
  open: boolean;
  targetEnabled: boolean; // true = SFW (Standard), false = NSFW (Privacy)
  onClose: () => void;
  onConfirm: () => void;
}

export default function ModeSwitchConfirmModal({
  open,
  targetEnabled,
  onClose,
  onConfirm,
}: ModeSwitchConfirmModalProps) {
  const { activeGameId, safeMode } = useAppStore();
  const [preview, setPreview] = useState<ModeSwitchPreviewResult | null>(null);
  const [isLoadingPreview, setIsLoadingPreview] = useState(false);

  // Fetch the backend text preview
  useEffect(() => {
    if (!open) {
      setPreview(null);
      return;
    }
    let isMounted = true;
    const fetchPreview = async () => {
      setIsLoadingPreview(true);
      try {
        const res = await invoke<ModeSwitchPreviewResult>('preview_mode_switch_enabled', {
          enabled: targetEnabled,
        });
        if (isMounted) setPreview(res);
      } catch (err) {
        console.error('Failed to preview mode switch:', err);
      } finally {
        if (isMounted) setIsLoadingPreview(false);
      }
    };
    fetchPreview();
    return () => {
      isMounted = false;
    };
  }, [open, targetEnabled]);

  // Queries for object lists
  const activeModsQuery = useActiveModsPreview(activeGameId, safeMode);
  const targetModsQuery = useCollectionPreview(preview?.target_coll_id ?? null, activeGameId);

  const leavingGroups = useMemo(() => {
    if (!activeModsQuery.data) return [];
    return groupMods(activeModsQuery.data);
  }, [activeModsQuery.data]);

  const targetGroups = useMemo(() => {
    if (!preview) return [];
    if (!preview.target_coll_id || !targetModsQuery.data) return []; // Empty state or still loading
    return groupMods(targetModsQuery.data);
  }, [preview, targetModsQuery.data]);

  const isDataLoading =
    isLoadingPreview ||
    (open && activeModsQuery.isLoading) ||
    (open && !!preview?.target_coll_id && targetModsQuery.isLoading);

  if (!open) return null;

  return createPortal(
    <dialog className="modal modal-open z-100">
      <div className="modal-box bg-base-200 border border-white/10 shadow-2xl max-w-5xl flex flex-col max-h-[85vh] p-0 overflow-hidden">
        {/* Header */}
        <div className="p-6 pb-4 border-b border-white/5 shrink-0 bg-base-300/30">
          <h3 className="font-bold text-lg flex items-center gap-2 text-warning mb-1">
            {!targetEnabled ? <ShieldAlert size={20} /> : <ShieldCheck size={20} />}
            Switch to {!targetEnabled ? 'Privacy Mode' : 'Standard Mode'}
          </h3>
          <p className="text-sm text-base-content/70">
            Review the changes to your active loadout before switching corridors.
          </p>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-hidden flex flex-col bg-base-100/50">
          {isDataLoading ? (
            <div className="flex flex-col h-full items-center justify-center p-8 text-base-content/50 min-h-75">
              <Loader2 size={32} className="animate-spin mb-4 opacity-50 text-warning" />
              <p>Loading preview...</p>
            </div>
          ) : (
            <div className="flex flex-col sm:flex-row h-full">
              {/* Left Column: Leaving */}
              <div className="flex-1 border-r border-white/5 flex flex-col min-h-0 sm:max-w-[50%]">
                <div className="p-4 border-b border-white/5 bg-error/5 shrink-0">
                  <h4 className="font-semibold text-sm flex justify-between items-center text-error/90 mb-1">
                    Leaving State
                    <span className="badge badge-sm badge-error badge-outline">Snapshot</span>
                  </h4>
                  <p className="text-xs text-base-content/50 break-all leading-tight">
                    Current Active Mods
                  </p>
                </div>
                <div className="p-4 overflow-y-auto custom-scrollbar flex-1">
                  <ModGroupList groups={leavingGroups} colorClass="text-error" />
                </div>
              </div>

              {/* Center Arrow */}
              <div className="hidden sm:flex items-center justify-center w-8 -mx-4 z-10 text-base-content/30 opacity-50 relative pointer-events-none">
                <div className="bg-base-200 rounded-full p-1 border border-white/10">
                  <ArrowRight size={20} />
                </div>
              </div>

              {/* Right Column: Target */}
              <div className="flex-1 flex flex-col min-h-0 sm:max-w-[50%]">
                <div className="p-4 border-b border-white/5 bg-success/5 shrink-0">
                  <h4 className="font-semibold text-sm flex justify-between items-center text-success/90 mb-1">
                    Destination State
                    <span className="badge badge-sm badge-success badge-outline">Restore</span>
                  </h4>
                  <p className="text-xs text-base-content/50 break-all leading-tight">
                    {preview?.target_name || 'Empty State'}
                  </p>
                </div>
                <div className="p-4 overflow-y-auto custom-scrollbar flex-1">
                  {preview?.target_coll_id ? (
                    <ModGroupList groups={targetGroups} colorClass="text-success" />
                  ) : (
                    <div className="text-center p-8 text-sm text-base-content/40 border border-white/5 border-dashed rounded-lg bg-base-100/10 m-2 mt-4">
                      Target state is empty (All Disabled)
                    </div>
                  )}
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="p-4 border-t border-white/5 shrink-0 bg-base-300/30">
          <div className="modal-action mt-0 gap-2">
            <button
              onClick={onClose}
              className="btn btn-ghost hover:bg-white/5"
              disabled={isDataLoading}
            >
              Cancel
            </button>
            <button
              onClick={onConfirm}
              className="btn btn-warning shadow-lg shadow-warning/10 font-bold tracking-wide"
              disabled={isDataLoading}
            >
              Continue {'->'}
            </button>
          </div>
        </div>
      </div>
    </dialog>,
    document.body,
  );
}
