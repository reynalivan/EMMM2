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
import { useState, useCallback, useEffect } from 'react';
import { useAppStore } from '../../../stores/useAppStore';
import { useSettings } from '../../../hooks/useSettings';
import TrashManagerModal from '../../modals/TrashManagerModal';
import LaunchBar from '../../LaunchBar/LaunchBar';
import PinEntryModal from '../../modals/PinEntryModal';
import { toast, useToastStore } from '../../../stores/useToastStore';

export default function GlobalActions() {
  const { workspaceView, setWorkspaceView, isPreviewOpen, togglePreview, safeMode, setSafeMode } =
    useAppStore();
  const { settings } = useSettings();
  const [trashOpen, setTrashOpen] = useState(false);
  const [pinModalOpen, setPinModalOpen] = useState(false);

  const handleToggleSafeMode = useCallback(async () => {
    // If currently Safe Mode ON and user wants to disable it (go to NSFW)
    if (safeMode) {
      if (settings?.safe_mode?.pin_hash) {
        setPinModalOpen(true);
      } else {
        const toastId = toast.info('Disabling Privacy Mode...', 0);
        try {
          await setSafeMode(false);
          useToastStore.getState().removeToast(toastId);
          toast.success('Privacy Mode Disabled');
        } catch (e) {
          useToastStore.getState().removeToast(toastId);
          toast.error(String(e));
        }
      }
    } else {
      // Unsafing back to Safe Mode is immediate, no PIN required
      const toastId = toast.info('Enabling Privacy Mode...', 0);
      try {
        await setSafeMode(true);
        useToastStore.getState().removeToast(toastId);
        toast.success('Privacy Mode Enabled');
      } catch (e) {
        useToastStore.getState().removeToast(toastId);
        toast.error(String(e));
      }
    }
  }, [safeMode, settings, setSafeMode]);

  // Epic 7 UI Req: Global Keyboard Shortcut Ctrl+Shift+S for switching SFW mode
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key.toLowerCase() === 's') {
        e.preventDefault();
        handleToggleSafeMode();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleToggleSafeMode]);

  return (
    <div className="flex items-center gap-2 md:gap-3">
      {/* Desktop Tools */}
      <div className="hidden md:flex items-center gap-1">
        <button
          className="btn btn-ghost btn-sm btn-square text-white/50 hover:text-warning hover:bg-white/5"
          title="Trash"
          onClick={() => setTrashOpen(true)}
        >
          <Trash2 size={18} />
        </button>
        <button
          className="btn btn-ghost btn-sm btn-square text-white/50 hover:text-white hover:bg-white/5"
          title="Refresh"
        >
          <RefreshCw size={18} />
        </button>
        <button
          className="btn btn-ghost btn-sm btn-square text-white/50 hover:text-white hover:bg-white/5"
          title="Settings"
          onClick={() => setWorkspaceView('settings')}
        >
          <Settings size={18} />
        </button>

        {/* Epic 7 Master Mode Switcher - Desktop */}
        <button
          className={`btn btn-sm btn-square transition-all duration-300 ${
            safeMode
              ? 'btn-ghost text-success bg-success/10 hover:bg-success/20'
              : 'btn-ghost text-error bg-error/10 hover:bg-error/20'
          }`}
          title={safeMode ? 'Safe Mode (ON) - Ctrl+Shift+S' : 'Privacy Mode (OFF) - Ctrl+Shift+S'}
          aria-label="Toggle Master Privacy Mode"
          onClick={handleToggleSafeMode}
        >
          {safeMode ? <ShieldCheck size={18} /> : <ShieldAlert size={18} />}
        </button>

        {/* Launch Bar (Epic 10) */}
        <LaunchBar />
      </div>

      {/* Trash Manager Modal */}
      <TrashManagerModal open={trashOpen} onClose={() => setTrashOpen(false)} />

      {/* Pin Entry Modal for Epic 7 Safe Mode */}
      <PinEntryModal
        open={pinModalOpen}
        onClose={() => setPinModalOpen(false)}
        onSuccess={async () => {
          const toastId = toast.info('Disabling Privacy Mode...', 0);
          try {
            await setSafeMode(false);
            setPinModalOpen(false);
            useToastStore.getState().removeToast(toastId);
            toast.success('Privacy Mode Disabled');
          } catch (e) {
            useToastStore.getState().removeToast(toastId);
            toast.error(String(e));
          }
        }}
      />

      {/* Mobile Menu Dropdown */}
      <div className="dropdown dropdown-end md:hidden">
        <div tabIndex={0} role="button" className="btn btn-sm btn-ghost btn-square text-white/70">
          <MoreVertical size={18} />
        </div>
        <ul
          tabIndex={0}
          className="dropdown-content z-100 menu p-2 shadow-2xl bg-base-100/90 backdrop-blur-xl rounded-box w-48 mt-2 border border-white/10"
        >
          <li>
            <a className="gap-2 hover:bg-white/5" onClick={() => setWorkspaceView('settings')}>
              <Settings size={16} /> Settings
            </a>
          </li>
          <li>
            <a className="gap-2 hover:bg-white/5" onClick={() => setTrashOpen(true)}>
              <Trash2 size={16} /> Trash
            </a>
          </li>
          <li>
            <a className="gap-2 hover:bg-white/5">
              <RefreshCw size={16} /> Refresh
            </a>
          </li>
          <div className="divider my-1 before:bg-white/5 after:bg-white/5"></div>
          <li>
            <a
              onClick={handleToggleSafeMode}
              className="gap-2 justify-between hover:bg-white/5"
              aria-label="Toggle Master Privacy Mode"
            >
              <span>{safeMode ? 'Safe Mode' : 'Privacy Mode'}</span>
              {safeMode ? (
                <ShieldCheck size={16} className="text-success" />
              ) : (
                <ShieldAlert size={16} className="text-error" />
              )}
            </a>
          </li>
        </ul>
      </div>

      <div className="w-px h-6 bg-white/5 mx-1 hidden md:block" />

      {/* Desktop Toggle Preview */}
      {workspaceView === 'mods' && (
        <button
          onClick={togglePreview}
          className={`btn btn-sm btn-square hidden md:flex ml-1 transition-all duration-300 ${isPreviewOpen ? 'btn-ghost text-primary bg-primary/10' : 'btn-ghost text-white/30 hover:text-white'}`}
          title={isPreviewOpen ? 'Hide Preview' : 'Show Preview'}
        >
          {isPreviewOpen ? <PanelRightClose size={18} /> : <PanelRightOpen size={18} />}
        </button>
      )}
    </div>
  );
}
