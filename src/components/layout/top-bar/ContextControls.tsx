import { ShieldCheck, ShieldAlert, Save, Layers, Loader2 } from 'lucide-react';
import { useAppStore } from '../../../stores/useAppStore';
import ApplyCollectionModal from '../../../features/collections/components/ApplyCollectionModal';
import SaveCollectionModal from '../../../features/collections/components/SaveCollectionModal';
import {
  useCollections,
  useCorridorRuntimeSnapshot,
} from '../../../features/collections/hooks/useCollections';
import { getCorridorStateName } from '../../../lib/corridorLabels';
import { useState } from 'react';
import { useSafeModeToggle } from '../../../hooks/useSafeModeToggle';
import PinEntryModal from '../../../features/safe-mode/PinEntryModal';
import ModeSwitchConfirmModal from '../../../features/safe-mode/ModeSwitchConfirmModal';

export default function ContextControls() {
  const { activeGameId, safeMode, setWorkspaceView, setWorkspaceSelectionForCorridor } =
    useAppStore();
  const {
    toggleSafeMode,
    handleConfirmSwitch,
    handlePinSuccess,
    confirmModalOpen,
    confirmTargetSafeMode,
    closeConfirmModal,
    pinModalOpen,
    closePinModal,
  } = useSafeModeToggle();
  const { data: collections = [], isLoading } = useCollections(activeGameId);
  const corridorOverviewQuery = useCorridorRuntimeSnapshot(activeGameId, safeMode);

  const [saveModalOpen, setSaveModalOpen] = useState(false);
  const [collectionToApply, setCollectionToApply] = useState<{ id: string; name: string } | null>(
    null,
  );

  const activeNamedCollectionId =
    corridorOverviewQuery.data?.state_kind === 'named'
      ? corridorOverviewQuery.data.active_collection_id
      : null;
  const triggerText =
    activeGameId && corridorOverviewQuery.status === 'pending'
      ? 'Loading...'
      : getCorridorStateName(corridorOverviewQuery.data?.state_name);
  const corridorCollections = collections.filter(
    (collection) => collection.is_safe_context === safeMode && !collection.is_last_unsaved,
  );

  const handleApplyClick = (e: React.MouseEvent, id: string, name: string) => {
    e.stopPropagation();
    if (!activeGameId) return;
    setWorkspaceSelectionForCorridor(activeGameId, safeMode, {
      kind: 'stored_collection',
      collection_id: id,
    });
    setCollectionToApply({ id, name });
    if (document.activeElement instanceof HTMLElement) {
      document.activeElement.blur();
    }
  };

  return (
    <>
      <div className="hidden lg:flex items-center gap-3 bg-base-100/30 p-1.5 rounded-full border border-white/5 backdrop-blur-md">
        <label
          className={`btn btn-xs btn-circle border-0 ${safeMode ? 'bg-success/20 text-success hover:bg-success/30' : 'bg-error/20 text-error hover:bg-error/30'}`}
          onClick={toggleSafeMode}
          title="Safe Mode Toggle"
        >
          {safeMode ? <ShieldCheck size={14} /> : <ShieldAlert size={14} />}
        </label>

        <div className="h-4 w-px bg-white/10" />

        <div className="dropdown dropdown-bottom dropdown-end">
          <div
            tabIndex={0}
            role="button"
            className={`px-3 py-1 rounded-full text-xs font-medium text-white/70 hover:text-white hover:bg-white/5 transition-all cursor-pointer flex items-center gap-2 max-w-37.5 ${activeNamedCollectionId === null && collections.length > 0 ? 'italic opacity-80' : ''}`}
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
                return corridorCollections.length === 0 ? (
                  <li className="disabled">
                    <span className="text-xs opacity-50 px-2">No collections saved</span>
                  </li>
                ) : (
                  <div className="max-h-[30vh] overflow-y-auto pr-1 custom-scrollbar">
                    {corridorCollections.map((c) => (
                      <li key={c.id}>
                        <button
                          className={`text-sm justify-between ${activeNamedCollectionId === c.id ? 'bg-white/10 text-white font-medium' : 'hover:bg-white/5'}`}
                          onClick={(e) => handleApplyClick(e, c.id, c.name)}
                        >
                          <span className="truncate max-w-32.5">{c.name}</span>
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

      {collectionToApply && (
        <ApplyCollectionModal
          collectionId={collectionToApply.id}
          collectionName={collectionToApply.name}
          onClose={() => setCollectionToApply(null)}
        />
      )}

      {/* Confirmation Modal for Corridor Switch */}
      <ModeSwitchConfirmModal
        open={confirmModalOpen}
        targetSafeMode={confirmTargetSafeMode}
        onClose={closeConfirmModal}
        onConfirm={handleConfirmSwitch}
      />

      {/* Pin Entry Modal for Context Controls Safe Mode */}
      <PinEntryModal
        open={pinModalOpen}
        onClose={closePinModal}
        onSuccess={async () => {
          handlePinSuccess();
        }}
      />

      {saveModalOpen && (
        <SaveCollectionModal mode="current_state" onClose={() => setSaveModalOpen(false)} />
      )}
    </>
  );
}
