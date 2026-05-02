import { useState, useRef } from 'react';
import { Trash2, Wrench, Eraser, RotateCcw, HardDrive } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useSettings } from '../../../hooks/useSettings';
import { commands } from '../../../lib/bindings';
import { useToastStore } from '../../../stores/useToastStore';
import { useAppStore } from '../../../stores/useAppStore';

export default function MaintenanceTab() {
  const { t } = useTranslation(['settings', 'common', 'layout']);
  const { runMaintenance } = useSettings();
  const { addToast } = useToastStore();
  const [isProcessing, setIsProcessing] = useState(false);
  const resetModalRef = useRef<HTMLDialogElement>(null);

  const handleEmptyTrash = async () => {
    if (!confirm(t('settings:maintenance.trash_confirm'))) return;

    setIsProcessing(true);
    try {
      await commands.emptyTrash();
      addToast('success', t('settings:maintenance.trash_success'));
    } catch (e) {
      console.error(e);
      addToast('error', t('settings:maintenance.trash_failed', { error: String(e) }));
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

  const handleClearCache = async () => {
    setIsProcessing(true);
    try {
      const count = await commands.clearOldThumbnails();
      addToast(
        'success',
        t('layout:maintenance.clear_success', {
          count,
        }),
      );
    } catch (e) {
      console.error(e);
      addToast('error', t('settings:maintenance.clear_failed', { error: String(e) }));
    } finally {
      setIsProcessing(false);
    }
  };

  const handleResetDatabase = async () => {
    resetModalRef.current?.close();
    setIsProcessing(true);
    try {
      await commands.resetDatabase();
      // Clear Zustand persisted state from localStorage
      localStorage.removeItem('vibecode-storage');
      addToast('success', t('settings:maintenance.reset_success'));
      window.location.reload();
    } catch (e) {
      console.error(e);
      addToast('error', t('settings:maintenance.reset_failed', { error: String(e) }));
      setIsProcessing(false);
    }
  };

  return (
    <div className="space-y-6">
      <div className="card bg-base-200 shadow-sm border border-base-300">
        <div className="card-body">
          <div className="flex items-start justify-between">
            <div>
              <h3 className="card-title text-lg flex items-center gap-2">
                <HardDrive className="text-warning" size={20} />
                {t('settings:maintenance.storage_title')}
              </h3>
              <p className="text-sm opacity-70 mt-1 max-w-2xl">
                {t('settings:maintenance.storage_desc')}
              </p>
            </div>
            <div className="card-actions shrink-0">
              <button
                className="btn btn-warning gap-2"
                onClick={() => useAppStore.getState().setWorkspaceView('storage-optimizer')}
              >
                <HardDrive size={18} /> {t('settings:maintenance.open_optimizer')}
              </button>
            </div>
          </div>
        </div>
      </div>

      <div className="card bg-base-200 shadow-sm border border-base-300">
        <div className="card-body">
          <h3 className="card-title text-lg flex items-center gap-2">
            <Trash2 className="text-error" size={20} />
            {t('settings:maintenance.trash_title')}
          </h3>
          <p className="text-sm opacity-70">{t('settings:maintenance.trash_desc')}</p>
          <div className="card-actions justify-end mt-4">
            <button
              className="btn btn-error btn-outline gap-2"
              onClick={handleEmptyTrash}
              disabled={isProcessing}
            >
              <Trash2 size={18} /> {t('settings:maintenance.empty_trash')}
            </button>
          </div>
        </div>
      </div>

      <div className="card bg-base-200 shadow-sm border border-base-300">
        <div className="card-body">
          <h3 className="card-title text-lg flex items-center gap-2">
            <Wrench className="text-primary" size={20} />
            {t('settings:maintenance.system_title')}
          </h3>
          <p className="text-sm opacity-70">{t('settings:maintenance.system_desc')}</p>
          <div className="card-actions justify-end mt-4">
            <button
              className="btn btn-primary gap-2"
              onClick={handleMaintenance}
              disabled={isProcessing}
            >
              <Wrench size={18} /> {t('settings:maintenance.run_maintenance')}
            </button>
          </div>
        </div>
      </div>

      <div className="card bg-base-200 shadow-sm border border-base-300">
        <div className="card-body">
          <h3 className="card-title text-lg flex items-center gap-2">
            <Eraser className="text-secondary" size={20} />
            {t('settings:maintenance.cache_title')}
          </h3>
          <p className="text-sm opacity-70">{t('settings:maintenance.cache_desc')}</p>
          <div className="card-actions justify-end mt-4">
            <button
              className="btn btn-secondary btn-outline gap-2"
              onClick={() => void handleClearCache()}
              disabled={isProcessing}
            >
              <Eraser size={18} /> {t('settings:maintenance.clear_cache')}
            </button>
          </div>
        </div>
      </div>

      {/* Reset Database — Danger Zone */}
      <div className="card bg-base-200 shadow-sm border border-error/30">
        <div className="card-body">
          <h3 className="card-title text-lg flex items-center gap-2">
            <RotateCcw className="text-error" size={20} />
            {t('settings:maintenance.danger_title')}
          </h3>
          <p className="text-sm opacity-70">{t('settings:maintenance.danger_desc')}</p>
          <p className="text-sm text-info mt-1">{t('settings:maintenance.danger_info')}</p>
          <div className="card-actions justify-end mt-4">
            <button
              id="btn-reset-database"
              className="btn btn-error gap-2"
              onClick={() => resetModalRef.current?.showModal()}
              disabled={isProcessing}
            >
              <RotateCcw size={18} /> {t('settings:maintenance.reset_btn')}
            </button>
          </div>
        </div>
      </div>

      {/* Confirmation Modal */}
      <dialog ref={resetModalRef} className="modal modal-bottom sm:modal-middle">
        <div className="modal-box">
          <h3 className="text-lg font-bold">{t('settings:maintenance.modal_title')}</h3>
          <p className="py-4">{t('settings:maintenance.modal_body')}</p>
          <p className="text-info text-sm">{t('settings:maintenance.danger_info')}</p>
          <p className="text-error text-sm font-semibold mt-2">
            {t('settings:maintenance.modal_error')}
          </p>
          <div className="modal-action">
            <form method="dialog">
              <button className="btn btn-ghost">{t('common:action.cancel')}</button>
            </form>
            <button id="btn-confirm-reset" className="btn btn-error" onClick={handleResetDatabase}>
              {t('settings:maintenance.confirm_reset')}
            </button>
          </div>
        </div>
        <form method="dialog" className="modal-backdrop bg-overlay-mask backdrop-blur-sm">
          <button>{t('common:action.close')}</button>
        </form>
      </dialog>
    </div>
  );
}
