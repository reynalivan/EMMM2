import { useEffect, useState } from 'react';
import { Download, FolderOpen, RefreshCw } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useSettings } from '@/entities/settings';
import { invoke } from '@tauri-apps/api/core';
import { SettingsRow, SettingsSection } from '../SettingsLayout';

type CatalogPackStatus = {
  state: 'not_installed' | 'ready' | 'partial' | 'invalid';
  pack_id: string | null;
  version: string | null;
  message: string | null;
  entries: number;
  missing_assets: number;
};

type CatalogPackRefreshResult = {
  state: CatalogPackStatus['state'];
  entries: number;
  thumbnails_applied: number;
  missing_assets: number;
  skipped_invalid_files: number;
};

type CatalogUpdateCheck = {
  state: 'update_available' | 'up_to_date';
  current_version: string | null;
  available_version: string | null;
  release_notes: string | null;
};

type CatalogUpdateInstallResult = {
  version: string;
  entries: number;
  missing_assets: number;
};

export default function CatalogTab() {
  const { t } = useTranslation(['settings', 'common']);
  const { settings, setCatalogAutoInstall } = useSettings();
  const [catalogPack, setCatalogPack] = useState<CatalogPackStatus | null>(null);
  const [isRefreshingCatalog, setIsRefreshingCatalog] = useState(false);
  const [catalogUpdate, setCatalogUpdate] = useState<CatalogUpdateCheck | null>(null);
  const [isCheckingCatalogUpdate, setIsCheckingCatalogUpdate] = useState(false);
  const [isInstallingCatalogUpdate, setIsInstallingCatalogUpdate] = useState(false);
  const [catalogUpdateError, setCatalogUpdateError] = useState<string | null>(null);

  const loadCatalogPackStatus = () => {
    void invoke<CatalogPackStatus>('get_catalog_pack_status')
      .then(setCatalogPack)
      .catch(() => {
        setCatalogPack({
          state: 'invalid',
          pack_id: null,
          version: null,
          message: t('general.catalog_assets.status_unavailable'),
          entries: 0,
          missing_assets: 0,
        });
      });
  };

  useEffect(loadCatalogPackStatus, [t]);

  const handleRefreshCatalog = async () => {
    setIsRefreshingCatalog(true);
    try {
      const result = await invoke<CatalogPackRefreshResult>('refresh_catalog_pack');
      setCatalogPack((current) => ({
        state: result.state,
        pack_id: current?.pack_id ?? null,
        version: current?.version ?? null,
        message: null,
        entries: result.entries,
        missing_assets: result.missing_assets,
      }));
    } catch {
      setCatalogUpdateError(t('general.catalog_assets.invalid'));
    } finally {
      setIsRefreshingCatalog(false);
      loadCatalogPackStatus();
    }
  };

  const handleCheckCatalogUpdate = async () => {
    setIsCheckingCatalogUpdate(true);
    setCatalogUpdateError(null);
    try {
      setCatalogUpdate(await invoke<CatalogUpdateCheck>('check_catalog_update'));
    } catch {
      setCatalogUpdateError(t('general.catalog_assets.update_check_failed'));
    } finally {
      setIsCheckingCatalogUpdate(false);
    }
  };

  const handleInstallCatalogUpdate = async () => {
    setIsInstallingCatalogUpdate(true);
    setCatalogUpdateError(null);
    try {
      const result = await invoke<CatalogUpdateInstallResult>('install_catalog_update');
      setCatalogPack((current) => ({
        state: result.missing_assets === 0 ? 'ready' : 'partial',
        pack_id: current?.pack_id ?? '3dm-catalog-asset',
        version: result.version,
        message: null,
        entries: result.entries,
        missing_assets: result.missing_assets,
      }));
      setCatalogUpdate({
        state: 'up_to_date',
        current_version: result.version,
        available_version: result.version,
        release_notes: null,
      });
    } catch {
      setCatalogUpdateError(t('general.catalog_assets.update_install_failed'));
    } finally {
      setIsInstallingCatalogUpdate(false);
      loadCatalogPackStatus();
    }
  };

  const catalogStatus =
    catalogPack?.state === 'ready' || catalogPack?.state === 'partial'
      ? t('general.catalog_assets.installed', {
          name: catalogPack.pack_id,
          version: catalogPack.version,
          entries: catalogPack.entries,
        })
      : catalogPack?.state === 'invalid'
        ? catalogPack.message || t('general.catalog_assets.invalid')
        : t('general.catalog_assets.not_installed');

  return (
    <div>
      <SettingsSection
        id="catalog-assets-heading"
        title={t('general.catalog_assets.title')}
        description={catalogStatus}
        action={
          <div className="flex flex-wrap gap-2">
            <button
              type="button"
              className="btn btn-sm btn-ghost gap-2"
              onClick={() => void invoke('open_catalog_pack_folder')}
            >
              <FolderOpen size={15} /> {t('general.catalog_assets.open_folder')}
            </button>
            <button
              type="button"
              className="btn btn-sm btn-primary gap-2"
              disabled={isRefreshingCatalog}
              onClick={() => void handleRefreshCatalog()}
            >
              <RefreshCw size={15} className={isRefreshingCatalog ? 'animate-spin' : undefined} />
              {t('general.catalog_assets.refresh')}
            </button>
          </div>
        }
      >
        <SettingsRow
          label={t('general.catalog_assets.updates_title')}
          description={t('general.catalog_assets.updates_desc')}
          control={
            <label className="label cursor-pointer gap-2 py-0" htmlFor="catalog-auto-install">
              <span className="label-text text-xs">{t('general.catalog_assets.auto_install')}</span>
              <input
                id="catalog-auto-install"
                type="checkbox"
                className="toggle toggle-sm toggle-primary"
                checked={settings?.catalog_updates?.auto_install ?? false}
                disabled={setCatalogAutoInstall.isPending || !settings}
                onChange={(event) => setCatalogAutoInstall.mutate(event.target.checked)}
              />
            </label>
          }
        />
        <div className="flex flex-wrap items-center justify-between gap-2 border-t border-base-300/70 pt-3">
          <p className="text-xs text-base-content/60" aria-live="polite">
            {catalogUpdateError ||
              (catalogUpdate?.state === 'update_available'
                ? t('general.catalog_assets.update_available', {
                    version: catalogUpdate.available_version,
                  })
                : catalogUpdate?.state === 'up_to_date'
                  ? t('general.catalog_assets.up_to_date')
                  : t('general.catalog_assets.update_idle'))}
          </p>
          <div className="flex gap-2">
            <button
              type="button"
              className="btn btn-sm btn-ghost gap-2"
              disabled={isCheckingCatalogUpdate || isInstallingCatalogUpdate}
              onClick={() => void handleCheckCatalogUpdate()}
            >
              <RefreshCw
                size={15}
                className={isCheckingCatalogUpdate ? 'animate-spin' : undefined}
              />
              {t('general.catalog_assets.check_update')}
            </button>
            {catalogUpdate?.state === 'update_available' && (
              <button
                type="button"
                className="btn btn-sm btn-primary gap-2"
                disabled={isInstallingCatalogUpdate}
                onClick={() => void handleInstallCatalogUpdate()}
              >
                <Download
                  size={15}
                  className={isInstallingCatalogUpdate ? 'animate-pulse' : undefined}
                />
                {t('general.catalog_assets.install_update', {
                  version: catalogUpdate.available_version,
                })}
              </button>
            )}
          </div>
        </div>
      </SettingsSection>
    </div>
  );
}
