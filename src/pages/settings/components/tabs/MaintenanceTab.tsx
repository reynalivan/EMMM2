import { formatAppError } from '../../../../shared/lib/appError';
import { useState, useRef } from 'react';
import { RotateCcw } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useSettings } from '@/entities/settings';
import { commands } from '../../../../shared/api/tauri/bindings';
import { useToastStore } from '@/shared/ui/toast';
import { useAppStore } from '@/app/store';
import { SettingsSection } from '../SettingsLayout';

export default function MaintenanceTab() {
  const { t } = useTranslation(['settings', 'common', 'layout']);
  const { runMaintenance } = useSettings();
  const { addToast } = useToastStore();
  const [isProcessing, setIsProcessing] = useState(false);
  const resetModalRef = useRef<HTMLDialogElement>(null);

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
      addToast('error', t('settings:maintenance.clear_failed', { error: formatAppError(e) }));
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
      addToast('error', t('settings:maintenance.reset_failed', { error: formatAppError(e) }));
      setIsProcessing(false);
    }
  };

  return (
    <div>
      <SettingsSection
        id="storage-optimizer-heading"
        title={t('settings:maintenance.storage_title')}
        description={t('settings:maintenance.storage_desc')}
        action={
          <button
            className="btn btn-outline btn-sm shrink-0 whitespace-nowrap"
            onClick={() => useAppStore.getState().setWorkspaceView('storage-optimizer')}
          >
            {t('settings:maintenance.open_optimizer')}
          </button>
        }
      />

      <SettingsSection
        id="system-maintenance-heading"
        title={t('settings:maintenance.system_title')}
        description={t('settings:maintenance.system_desc')}
        action={
          <button
            className="btn btn-primary btn-sm shrink-0 whitespace-nowrap"
            onClick={handleMaintenance}
            disabled={isProcessing}
          >
            {t('settings:maintenance.run_maintenance')}
          </button>
        }
      />

      <SettingsSection
        id="image-cache-heading"
        title={t('settings:maintenance.cache_title')}
        description={t('settings:maintenance.cache_desc')}
        action={
          <button
            className="btn btn-outline btn-sm shrink-0 whitespace-nowrap"
            onClick={() => void handleClearCache()}
            disabled={isProcessing}
          >
            {t('settings:maintenance.clear_cache')}
          </button>
        }
      />

      <SettingsSection
        id="reset-heading"
        title={t('settings:maintenance.danger_title')}
        description={`${t('settings:maintenance.danger_desc')} ${t('settings:maintenance.danger_info')}`}
        className="border-error/30"
        action={
          <button
            id="btn-reset-database"
            className="btn btn-error btn-sm shrink-0 gap-2 whitespace-nowrap"
            onClick={() => resetModalRef.current?.showModal()}
            disabled={isProcessing}
          >
            <RotateCcw size={16} />
            {t('settings:maintenance.reset_btn')}
          </button>
        }
      />

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
