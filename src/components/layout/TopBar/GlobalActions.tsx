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
import { useAppStore } from '../../../stores/useAppStore';
import TrashManagerModal from '../../modals/TrashManagerModal';
import LaunchBar from '../../LaunchBar/LaunchBar';

export default function GlobalActions() {
  const { workspaceView, setWorkspaceView, isPreviewOpen, togglePreview, safeMode, setSafeMode } =
    useAppStore();
  const [trashOpen, setTrashOpen] = useState(false);

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
        {/* Launch Bar (Epic 10) */}
        <LaunchBar />
      </div>

      {/* Trash Manager Modal */}
      <TrashManagerModal open={trashOpen} onClose={() => setTrashOpen(false)} />

      {/* Mobile Menu Dropdown */}
      <div className="dropdown dropdown-end md:hidden">
        <div tabIndex={0} role="button" className="btn btn-sm btn-ghost btn-square text-white/70">
          <MoreVertical size={18} />
        </div>
        <ul
          tabIndex={0}
          className="dropdown-content z-1 menu p-2 shadow-2xl bg-base-100/90 backdrop-blur-xl rounded-box w-48 mt-2 border border-white/10"
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
              onClick={() => setSafeMode(!safeMode)}
              className="gap-2 justify-between hover:bg-white/5"
            >
              <span>Safe Mode</span>
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
