import { useMemo } from 'react';
import { createPortal } from 'react-dom';
import { invoke } from '@tauri-apps/api/core';
import { ShieldCheck, ShieldAlert, Loader2, ArrowRight } from 'lucide-react';
import { useQuery } from '@tanstack/react-query';
import { useAppStore } from '../../stores/useAppStore';
import { ModGroupList } from '../collections/components/ModGroupList';
import { groupMods } from '../collections/utils/groupMods';
import type { CollectionPreviewMod } from '../../types/collection';

/** Backend response from preview_corridor_switch */
interface CorridorPreview {
  leaving_mods: CollectionPreviewMod[];
  target_mods: CollectionPreviewMod[];
  target_description: string;
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
  const { activeGameId } = useAppStore();

  // Single consolidated query — replaces the old 3-query pattern
  const { data: preview, isLoading } = useQuery<CorridorPreview>({
    queryKey: ['corridor-preview', activeGameId, targetEnabled],
    queryFn: () => invoke<CorridorPreview>('preview_corridor_switch', { targetEnabled }),
    enabled: open && !!activeGameId,
    staleTime: 0, // Always refetch when modal opens
  });

  const leavingGroups = useMemo(() => {
    if (!preview?.leaving_mods) return [];
    return groupMods(preview.leaving_mods);
  }, [preview]);

  const targetGroups = useMemo(() => {
    if (!preview?.target_mods?.length) return [];
    return groupMods(preview.target_mods);
  }, [preview]);

  if (!open) return null;

  return createPortal(
    <dialog className="modal modal-open z-1000">
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
        <div className="flex-1 overflow-hidden flex flex-col bg-base-100/50 min-h-0">
          {isLoading ? (
            <div className="flex-1 flex flex-col items-center justify-center p-8 text-base-content/50 min-h-75">
              <Loader2 size={32} className="animate-spin mb-4 opacity-50 text-warning" />
              <p>Loading preview...</p>
            </div>
          ) : (
            <div className="flex-1 flex flex-col sm:flex-row min-h-0">
              {/* Left Column: Leaving */}
              <div className="flex-1 border-r border-white/5 flex flex-col min-h-0 sm:max-w-[50%]">
                <div
                  className={`p-4 border-b border-white/5 ${targetEnabled ? 'bg-error/5' : 'bg-success/5'} shrink-0`}
                >
                  <h4
                    className={`font-semibold text-sm flex justify-between items-center ${targetEnabled ? 'text-error/90' : 'text-success/90'} mb-1`}
                  >
                    Leaving State ({targetEnabled ? 'Unsafe Mode' : 'Safe Mode'})
                    <span
                      className={`badge badge-sm ${targetEnabled ? 'badge-error' : 'badge-success'} badge-outline`}
                    >
                      Snapshot
                    </span>
                  </h4>
                  <p className="text-xs text-base-content/50 break-all leading-tight">
                    Current Active Mods
                  </p>
                </div>
                <div className="p-4 overflow-y-auto custom-scrollbar flex-1">
                  <ModGroupList
                    groups={leavingGroups}
                    colorClass={targetEnabled ? 'text-error' : 'text-success'}
                  />
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
                <div
                  className={`p-4 border-b border-white/5 ${targetEnabled ? 'bg-success/5' : 'bg-error/5'} shrink-0`}
                >
                  <h4
                    className={`font-semibold text-sm flex justify-between items-center ${targetEnabled ? 'text-success/90' : 'text-error/90'} mb-1`}
                  >
                    Destination State ({targetEnabled ? 'Safe Mode' : 'Unsafe Mode'})
                    <span
                      className={`badge badge-sm ${targetEnabled ? 'badge-success' : 'badge-error'} badge-outline`}
                    >
                      Restore
                    </span>
                  </h4>
                  <p className="text-xs text-base-content/50 break-all leading-tight">
                    {preview?.target_description || 'Empty State'}
                  </p>
                </div>
                <div className="p-4 overflow-y-auto custom-scrollbar flex-1">
                  {targetGroups.length > 0 ? (
                    <ModGroupList
                      groups={targetGroups}
                      colorClass={targetEnabled ? 'text-success' : 'text-error'}
                    />
                  ) : (
                    <div
                      className={`text-center p-8 text-sm text-base-content/40 border ${targetEnabled ? 'border-success/20' : 'border-error/20'} border-dashed rounded-lg bg-base-100/10 m-2 mt-4`}
                    >
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
              disabled={isLoading}
            >
              Cancel
            </button>
            <button
              onClick={onConfirm}
              className="btn btn-warning shadow-lg shadow-warning/10 font-bold tracking-wide"
              disabled={isLoading}
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
