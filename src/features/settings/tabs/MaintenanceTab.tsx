import { useState } from 'react';
import { Trash2, Wrench, Eraser } from 'lucide-react';
import { useSettings } from '../../../hooks/useSettings';
import { invoke } from '@tauri-apps/api/core';
import { useToastStore } from '../../../stores/useToastStore';

export default function MaintenanceTab() {
  const { runMaintenance } = useSettings();
  const { addToast } = useToastStore();
  const [isProcessing, setIsProcessing] = useState(false);

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
    </div>
  );
}
