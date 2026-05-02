import React from 'react';
import { AlertCircle, RefreshCw, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { closeWorkspaceDialog } from '../../features/workspace-runtime/state/workspaceDialogs';
import { useWorkspaceRuntimeSelector } from '../../features/workspace-runtime/state/workspaceStoreBridge';

export const FileInUseDialog: React.FC = () => {
  const { t } = useTranslation('common');
  const runtimeDialogState = useWorkspaceRuntimeSelector((state) => state.dialogState);
  const runtimeFileInUse = runtimeDialogState.kind === 'fileInUse' ? runtimeDialogState.data : null;
  const open = runtimeFileInUse !== null;
  const path = runtimeFileInUse?.path ?? null;
  const processes = runtimeFileInUse?.processes ?? [];
  const onRetry = runtimeFileInUse?.onRetry;

  const closeDialog = () => {
    closeWorkspaceDialog('fileInUse');
  };

  if (!open) return null;

  const handleRetry = () => {
    onRetry?.();
    closeDialog();
  };

  const folderName = path?.split(/[/\\]/).pop() || t('file_in_use.folder_fallback');

  return (
    <div className="modal modal-open">
      <div className="modal-box max-w-md border border-warning/20 bg-base-200">
        <div className="flex items-center gap-3 mb-4">
          <div className="btn btn-circle btn-warning btn-sm no-animation pointer-events-none">
            <AlertCircle className="w-5 h-5" />
          </div>
          <h3 className="text-xl font-bold text-base-content">{t('file_in_use.title')}</h3>
        </div>

        <div className="space-y-4">
          <p className="text-sm text-base-content/70">
            {t('file_in_use.description', { folderName })}
          </p>

          <div className="bg-base-300 rounded-lg p-3 max-h-40 overflow-y-auto border border-base-content/5">
            <ul className="list-disc list-inside space-y-1 text-sm font-medium">
              {processes.map((proc, idx) => (
                <li key={idx} className="text-base-content">
                  {proc === 'explorer.exe' ? t('file_in_use.windows_explorer') : proc}
                </li>
              ))}
            </ul>
          </div>

          <div className="alert alert-info py-2 shadow-sm text-xs">
            <AlertCircle className="w-4 h-4 shrink-0" />
            <span>{t('file_in_use.hint')}</span>
          </div>
        </div>

        <div className="modal-action flex gap-2">
          <button className="btn btn-ghost btn-sm" onClick={closeDialog}>
            <X className="w-4 h-4 mr-1" />
            {t('actions.cancel')}
          </button>
          <button className="btn btn-warning btn-sm" onClick={handleRetry}>
            <RefreshCw className="w-4 h-4 mr-1" />
            {t('actions.retry')}
          </button>
        </div>
      </div>
      <div className="modal-backdrop bg-black/60 backdrop-blur-sm" onClick={closeDialog}></div>
    </div>
  );
};
