import { RefreshCw, Download, CheckCircle, AlertTriangle, Database } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useAppUpdater } from '../hooks/useAppUpdater';
import { useMetadataSyncMutation } from '../hooks/useMetadataSync';
import { useToastStore } from '../../../stores/useToastStore';
import { getVersion } from '@tauri-apps/api/app';
import { useEffect, useState } from 'react';
import { formatBytes } from '../../../utils/formatters';

export default function UpdateTab() {
  const { t } = useTranslation(['settings', 'common']);
  const { addToast } = useToastStore();
  const [appVersion, setAppVersion] = useState('...');
  const {
    update,
    isChecking,
    isInstalling,
    progress,
    error: updateError,
    checkForUpdate,
    downloadAndInstall,
    dismiss,
  } = useAppUpdater();

  const metadataSync = useMetadataSyncMutation();

  useEffect(() => {
    getVersion()
      .then(setAppVersion)
      .catch(() => setAppVersion('unknown'));
  }, []);

  const handleCheckUpdate = async () => {
    await checkForUpdate();
  };

  const handleMetadataSync = () => {
    metadataSync.mutate(undefined, {
      onSuccess: (data) => {
        if (data.updated) {
          addToast('success', t('settings:update.sync_success', { version: data.version }));
        } else {
          addToast('info', t('settings:update.sync_up_to_date'));
        }
      },
      onError: (err) => {
        addToast('error', t('settings:update.sync_failed', { error: String(err) }));
      },
    });
  };

  const progressPercent =
    progress && progress.total ? Math.round((progress.downloaded / progress.total) * 100) : null;

  return (
    <div className="space-y-6">
      {/* Current Version Info */}
      <div className="card bg-base-200 shadow-sm border border-base-300">
        <div className="card-body">
          <h3 className="card-title text-lg flex items-center gap-2">
            <RefreshCw className="text-primary" size={20} />
            {t('settings:update.title')}
          </h3>
          <p className="text-sm opacity-70">
            {t('settings:update.current_version', { version: appVersion })}
          </p>

          {/* Update Available Card */}
          {update && (
            <div className="alert alert-info mt-4">
              <Download size={18} />
              <div className="flex-1">
                <p className="font-semibold">
                  {t('settings:update.available', { version: update.version })}
                </p>
                {update.body && (
                  <p className="text-sm opacity-80 mt-1 whitespace-pre-wrap">{update.body}</p>
                )}
              </div>
            </div>
          )}

          {/* Progress Bar */}
          {progress && (
            <div className="mt-4">
              <div className="flex justify-between text-sm mb-1">
                <span>
                  {t('settings:update.downloading', {
                    downloaded: formatBytes(progress.downloaded),
                    total: progress.total ? formatBytes(progress.total) : '?',
                  })}
                </span>
                {progressPercent !== null && <span>{progressPercent}%</span>}
              </div>
              <progress
                className="progress progress-primary w-full"
                value={progressPercent ?? undefined}
                max={100}
              />
            </div>
          )}

          {/* Update Error */}
          {updateError && (
            <div className="alert alert-error mt-4">
              <AlertTriangle size={18} />
              <span>{updateError}</span>
              <button className="btn btn-ghost btn-xs" onClick={dismiss}>
                {t('common:action.dismiss')}
              </button>
            </div>
          )}

          {/* No update available feedback */}
          {!update && !isChecking && !updateError && !progress && (
            <div className="flex items-center gap-2 text-sm text-success mt-2">
              <CheckCircle size={16} />
              <span>{t('settings:update.latest')}</span>
            </div>
          )}

          <div className="card-actions justify-end mt-4">
            {update && !isInstalling && (
              <button className="btn btn-primary gap-2" onClick={() => void downloadAndInstall()}>
                <Download size={18} /> {t('settings:update.install_btn')}
              </button>
            )}
            <button
              className="btn btn-outline gap-2"
              onClick={() => void handleCheckUpdate()}
              disabled={isChecking || isInstalling}
            >
              <RefreshCw size={18} className={isChecking ? 'animate-spin' : ''} />
              {isChecking ? t('settings:update.checking') : t('settings:update.check_btn')}
            </button>
          </div>
        </div>
      </div>

      {/* Metadata Sync */}
      <div className="card bg-base-200 shadow-sm border border-base-300">
        <div className="card-body">
          <h3 className="card-title text-lg flex items-center gap-2">
            <Database className="text-secondary" size={20} />
            {t('settings:update.metadata_title')}
          </h3>
          <p className="text-sm opacity-70">{t('settings:update.metadata_desc')}</p>

          <div className="card-actions justify-end mt-4">
            <button
              className="btn btn-secondary btn-outline gap-2"
              onClick={handleMetadataSync}
              disabled={metadataSync.isPending}
            >
              <RefreshCw size={18} className={metadataSync.isPending ? 'animate-spin' : ''} />
              {metadataSync.isPending
                ? t('settings:update.syncing')
                : t('settings:update.sync_btn')}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
