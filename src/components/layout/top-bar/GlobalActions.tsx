import {
  RefreshCw,
  Settings,
  MoreVertical,
  PanelRightClose,
  PanelRightOpen,
  ShieldCheck,
  ShieldAlert,
  Trash2,
} from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useAppStore } from '../../../stores/useAppStore';
import TrashManagerModal from '../../../features/file-management/TrashManagerModal';
import LaunchBar from '../../../features/launch-bar/LaunchBar';
import PinEntryModal from '../../../features/safe-mode/PinEntryModal';
import ModeSwitchConfirmModal from '../../../features/safe-mode/ModeSwitchConfirmModal';
import { useSafeModeToggle } from '../../../features/collections/hooks/useSafeModeToggle';

export default function GlobalActions() {
  const { t } = useTranslation('layout');
  const { workspaceView, setWorkspaceView, isPreviewOpen, togglePreview } = useAppStore();
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
  const [trashOpen, setTrashOpen] = useState(false);

  return (
    <div className="flex items-center gap-2 md:gap-3">
      {/* Desktop Tools */}
      <div className="hidden md:flex items-center gap-1">
        <button
          className="btn btn-ghost btn-sm btn-square text-base-content/50 hover:text-warning hover:bg-base-content/10"
          title={t('actions.trash')}
          onClick={() => setTrashOpen(true)}
        >
          <Trash2 size={18} />
        </button>
        <button
          className="btn btn-ghost btn-sm btn-square text-base-content/50 hover:text-primary hover:bg-base-content/10"
          title={t('actions.refresh')}
        >
          <RefreshCw size={18} />
        </button>
        <button
          className="btn btn-ghost btn-sm btn-square text-base-content/50 hover:text-primary hover:bg-base-content/10"
          title={t('actions.settings')}
          onClick={() => setWorkspaceView('settings')}
        >
          <Settings size={18} />
        </button>

        {/* Launch Bar (Epic 10) */}
        <LaunchBar />
      </div>

      {/* Trash Manager Modal */}
      <TrashManagerModal open={trashOpen} onClose={() => setTrashOpen(false)} />

      {/* Confirmation Modal for Corridor Switch */}
      <ModeSwitchConfirmModal
        open={confirmModalOpen}
        targetSafeMode={confirmTargetSafeMode}
        onClose={closeConfirmModal}
        onConfirm={handleConfirmSwitch}
      />

      {/* Pin Entry Modal for Epic 7 Safe Mode */}
      <PinEntryModal
        open={pinModalOpen}
        onClose={closePinModal}
        onSuccess={async () => {
          handlePinSuccess();
        }}
      />

      {/* Mobile Menu Dropdown */}
      <div className="dropdown dropdown-end md:hidden">
        <div
          tabIndex={0}
          role="button"
          className="btn btn-sm btn-ghost btn-square text-base-content/70"
        >
          <MoreVertical size={18} />
        </div>
        <ul
          tabIndex={0}
          className="dropdown-content z-100 menu p-2 shadow-2xl bg-base-100/95 backdrop-blur-xl rounded-box w-48 mt-2 border border-base-content/10"
        >
          <li>
            <a
              className="gap-2 hover:bg-base-content/10"
              onClick={() => setWorkspaceView('settings')}
            >
              <Settings size={16} /> {t('actions.settings')}
            </a>
          </li>
          <li>
            <a className="gap-2 hover:bg-base-content/10" onClick={() => setTrashOpen(true)}>
              <Trash2 size={16} /> {t('actions.trash')}
            </a>
          </li>
          <li>
            <a className="gap-2 hover:bg-base-content/10">
              <RefreshCw size={16} /> {t('actions.refresh')}
            </a>
          </li>
          <div className="divider my-1 before:bg-base-content/10 after:bg-base-content/10"></div>
          <li>
            <a
              onClick={toggleSafeMode}
              className="gap-2 justify-between hover:bg-base-content/10"
              aria-label={t('actions.privacy_toggle')}
            >
              <span>{safeMode ? t('actions.safe_mode') : t('actions.privacy_mode')}</span>
              {safeMode ? (
                <ShieldCheck size={16} className="text-success" />
              ) : (
                <ShieldAlert size={16} className="text-error" />
              )}
            </a>
          </li>
        </ul>
      </div>

      <div className="w-px h-6 bg-base-content/10 mx-1 hidden md:block" />

      {/* Desktop Toggle Preview */}
      {workspaceView === 'mods' && (
        <button
          onClick={togglePreview}
          className={`btn btn-sm btn-square hidden md:flex ml-1 transition-all duration-300 ${isPreviewOpen ? 'btn-ghost text-primary bg-primary/10' : 'btn-ghost text-base-content/30 hover:text-primary'}`}
          title={isPreviewOpen ? t('actions.hide_preview') : t('actions.show_preview')}
        >
          {isPreviewOpen ? <PanelRightClose size={18} /> : <PanelRightOpen size={18} />}
        </button>
      )}
    </div>
  );
}
