import React from 'react';
import { AlertCircle, RefreshCw, X } from 'lucide-react';
import { useAppStore } from '../../stores/useAppStore';

export const FileInUseDialog: React.FC = () => {
  const { fileInUseDialog, closeFileInUseDialog } = useAppStore();
  const { open, path, processes, onRetry } = fileInUseDialog;

  if (!open) return null;

  const handleRetry = () => {
    onRetry?.();
    closeFileInUseDialog();
  };

  const folderName = path?.split(/[/\\]/).pop() || 'folder';

  return (
    <div className="modal modal-open">
      <div className="modal-box max-w-md border border-warning/20 bg-base-200">
        <div className="flex items-center gap-3 mb-4">
          <div className="btn btn-circle btn-warning btn-sm no-animation pointer-events-none">
            <AlertCircle className="w-5 h-5" />
          </div>
          <h3 className="text-xl font-bold text-base-content">File In Use</h3>
        </div>

        <div className="space-y-4">
          <p className="text-sm text-base-content/70">
            Cannot modify <span className="text-warning font-mono">"{folderName}"</span> because it is currently in use by the following programs:
          </p>

          <div className="bg-base-300 rounded-lg p-3 max-h-40 overflow-y-auto border border-base-content/5">
            <ul className="list-disc list-inside space-y-1 text-sm font-medium">
              {processes.map((proc, idx) => (
                <li key={idx} className="text-base-content">
                  {proc === 'explorer.exe' ? 'Windows Explorer' : proc}
                </li>
              ))}
            </ul>
          </div>

          <div className="alert alert-info py-2 shadow-sm text-xs">
            <AlertCircle className="w-4 h-4 shrink-0" />
            <span>Please close these programs and try again.</span>
          </div>
        </div>

        <div className="modal-action flex gap-2">
          <button
            className="btn btn-ghost btn-sm"
            onClick={closeFileInUseDialog}
          >
            <X className="w-4 h-4 mr-1" />
            Cancel
          </button>
          <button
            className="btn btn-warning btn-sm"
            onClick={handleRetry}
          >
            <RefreshCw className="w-4 h-4 mr-1" />
            Retry
          </button>
        </div>
      </div>
      <div className="modal-backdrop bg-black/60 backdrop-blur-sm" onClick={closeFileInUseDialog}></div>
    </div>
  );
};
