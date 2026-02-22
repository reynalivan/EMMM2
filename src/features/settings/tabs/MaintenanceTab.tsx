import { useState, useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import { Trash2, Wrench, Eraser, RotateCcw } from 'lucide-react';
import { useSettings } from '../../../hooks/useSettings';
import { invoke } from '@tauri-apps/api/core';
import { useToastStore } from '../../../stores/useToastStore';

export default function MaintenanceTab() {
  const { runMaintenance } = useSettings();
  const { addToast } = useToastStore();
  const navigate = useNavigate();
  const [isProcessing, setIsProcessing] = useState(false);
  const resetModalRef = useRef<HTMLDialogElement>(null);

  const handleEmptyTrash = async () => {
    if (!confirm('Are you sure you want to permanently delete all items in the Trash?')) return;

    setIsProcessing(true);
    try {
      await invoke('empty_trash');
      addToast('success', 'Trash Emptied: All items deleted.');
    } catch (e) {
      console.error(e);
      addToast('error', `Failed: ${String(e)}`);
    } finally {
      setIsProcessing(false);
    }
  };

  const handleMaintenance = () => {
    setIsProcessing(true);
    runMaintenance(undefined, {
      onSuccess: () => setIsProcessing(false),
      onError: () => setIsProcessing(false),
    });
  };

  const handleResetDatabase = async () => {
    resetModalRef.current?.close();
    setIsProcessing(true);
    try {
      await invoke('reset_database');
      // Clear Zustand persisted state from localStorage
      localStorage.removeItem('vibecode-storage');
      addToast('success', 'Database reset. Redirecting to setup...');
      navigate('/welcome', { replace: true });
    } catch (e) {
      console.error(e);
      addToast('error', `Reset Failed: ${String(e)}`);
      setIsProcessing(false);
    }
  };

  return (
    <div className="space-y-6">
      <div className="card bg-base-200 shadow-sm border border-base-300">
        <div className="card-body">
          <h3 className="card-title text-lg flex items-center gap-2">
            <Trash2 className="text-error" size={20} />
            Trash Management
          </h3>
          <p className="text-sm opacity-70">
            Permanently remove deleted mods to free up disk space.
          </p>
          <div className="card-actions justify-end mt-4">
            <button
              className="btn btn-error btn-outline gap-2"
              onClick={handleEmptyTrash}
              disabled={isProcessing}
            >
              <Trash2 size={18} /> Empty Trash
            </button>
          </div>
        </div>
      </div>

      <div className="card bg-base-200 shadow-sm border border-base-300">
        <div className="card-body">
          <h3 className="card-title text-lg flex items-center gap-2">
            <Wrench className="text-primary" size={20} />
            System Maintenance
          </h3>
          <p className="text-sm opacity-70">
            Run automated cleanup tasks, verify database integrity, and prune orphaned metadata.
          </p>
          <div className="card-actions justify-end mt-4">
            <button
              className="btn btn-primary gap-2"
              onClick={handleMaintenance}
              disabled={isProcessing}
            >
              <Wrench size={18} /> Run Maintenance
            </button>
          </div>
        </div>
      </div>

      <div className="card bg-base-200 shadow-sm border border-base-300 opacity-50 cursor-not-allowed">
        <div className="card-body">
          <h3 className="card-title text-lg flex items-center gap-2">
            <Eraser size={20} />
            Clear Image Cache
          </h3>
          <p className="text-sm opacity-70">
            Remove generated thumbnails to save space. They will be regenerated as needed.
          </p>
          <div className="card-actions justify-end mt-4">
            <button className="btn btn-neutral gap-2" disabled>
              Coming Soon
            </button>
          </div>
        </div>
      </div>

      {/* Reset Database — Danger Zone */}
      <div className="card bg-base-200 shadow-sm border border-error/30">
        <div className="card-body">
          <h3 className="card-title text-lg flex items-center gap-2">
            <RotateCcw className="text-error" size={20} />
            Reset Application Setup
          </h3>
          <p className="text-sm opacity-70">
            Clear all game registrations, mod metadata, and settings from the database. The
            application will return to the initial setup screen. A backup of the database will be
            saved to the trash folder.
          </p>
          <p className="text-sm text-info mt-1">
            Your mod files and folders on disk will not be deleted.
          </p>
          <div className="card-actions justify-end mt-4">
            <button
              id="btn-reset-database"
              className="btn btn-error gap-2"
              onClick={() => resetModalRef.current?.showModal()}
              disabled={isProcessing}
            >
              <RotateCcw size={18} /> Reset & Re-Setup
            </button>
          </div>
        </div>
      </div>

      {/* Confirmation Modal */}
      <dialog ref={resetModalRef} className="modal modal-bottom sm:modal-middle">
        <div className="modal-box">
          <h3 className="text-lg font-bold">Are you sure?</h3>
          <p className="py-4">
            This will clear all game registrations, mod metadata, collections, and settings from the
            database. A backup will be saved to the trash folder before clearing.
          </p>
          <p className="text-info text-sm">
            Your mod files and folders on disk will not be affected.
          </p>
          <p className="text-error text-sm font-semibold mt-2">
            You will need to set up the application again.
          </p>
          <div className="modal-action">
            <form method="dialog">
              <button className="btn btn-ghost">Cancel</button>
            </form>
            <button id="btn-confirm-reset" className="btn btn-error" onClick={handleResetDatabase}>
              Yes, Reset Everything
            </button>
          </div>
        </div>
        <form method="dialog" className="modal-backdrop">
          <button>close</button>
        </form>
      </dialog>
    </div>
  );
}
