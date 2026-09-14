import { useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Download, FileArchive, FolderOpen, RefreshCw, Search } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useActiveGame } from '@/entities/game';
import { useAppStore } from '@/app/store';
import { commands } from '@/shared/api/tauri/bindings';
import { publishQueryScopes } from '@/shared/lib/queryRefresh';
import type {
  CatalogImportPreview,
  CatalogPackRefreshResult,
  CatalogPackStatus,
  CatalogUpdateCheck,
  CatalogUpdateInstallResult,
} from '@/shared/api/tauri/bindings.gen';
import { SettingsRow, SettingsSection } from '../SettingsLayout';

const catalogPackKeys = {
  status: ['catalog-pack', 'status'] as const,
};

export default function CatalogTab() {
  const { t } = useTranslation(['settings', 'common']);
  const { activeGame } = useActiveGame();
  const activeGameId = useAppStore((state) => state.activeGameId);
  const queryClient = useQueryClient();
  const [catalogUpdate, setCatalogUpdate] = useState<CatalogUpdateCheck | null>(null);
  const [catalogError, setCatalogError] = useState<string | null>(null);
  const [githubUrl, setGithubUrl] = useState('');
  const [catalogPreview, setCatalogPreview] = useState<CatalogImportPreview | null>(null);

  const catalogPackQuery = useQuery<CatalogPackStatus>({
    queryKey: catalogPackKeys.status,
    queryFn: () => commands.getCatalogPackStatus(),
    networkMode: 'always',
    staleTime: 30_000,
  });
  const catalogPack =
    catalogPackQuery.data ??
    (catalogPackQuery.isError
      ? ({
          state: 'invalid',
          pack_id: null,
          version: null,
          message: t('general.catalog_assets.status_unavailable'),
          entries: 0,
        } satisfies CatalogPackStatus)
      : null);

  const refreshCatalogMutation = useMutation<CatalogPackRefreshResult, unknown>({
    mutationFn: () => commands.refreshCatalogPack(),
    networkMode: 'always',
    onSuccess: async () => {
      await publishQueryScopes(queryClient, ['catalogPack']);
    },
    onError: () => setCatalogError(t('general.catalog_assets.invalid')),
  });
  const checkCatalogUpdateMutation = useMutation<CatalogUpdateCheck, unknown>({
    mutationFn: () => commands.checkCatalogUpdate(),
    networkMode: 'always',
    onMutate: () => setCatalogError(null),
    onSuccess: setCatalogUpdate,
    onError: () => setCatalogError(t('general.catalog_assets.update_check_failed')),
  });
  const installCatalogUpdateMutation = useMutation<CatalogUpdateInstallResult, unknown>({
    mutationFn: () => commands.installCatalogUpdate(),
    networkMode: 'always',
    onMutate: () => setCatalogError(null),
    onSuccess: async (result) => {
      setCatalogUpdate({
        state: 'up_to_date',
        current_version: result.version,
        available_version: result.version,
        release_notes: null,
      });
      await publishQueryScopes(queryClient, ['catalogPack']);
    },
    onError: () => setCatalogError(t('general.catalog_assets.update_install_failed')),
  });
  const previewGithubImportMutation = useMutation<CatalogImportPreview, unknown>({
    mutationFn: () => commands.previewCatalogGithubImport(githubUrl.trim()),
    networkMode: 'always',
    onMutate: () => setCatalogError(null),
    onSuccess: setCatalogPreview,
    onError: () => setCatalogError(t('general.catalog_assets.github_preview_failed')),
  });
  const previewLocalImportMutation = useMutation<CatalogImportPreview | null, unknown>({
    mutationFn: () => commands.previewCatalogLocalImport(),
    networkMode: 'always',
    onMutate: () => setCatalogError(null),
    onSuccess: (preview) => {
      if (preview) setCatalogPreview(preview);
    },
    onError: () => setCatalogError(t('general.catalog_assets.local_preview_failed')),
  });
  const installCatalogImportMutation = useMutation<CatalogPackRefreshResult, unknown>({
    mutationFn: () => commands.installCatalogImport(catalogPreview?.stagingToken ?? ''),
    networkMode: 'always',
    onMutate: () => setCatalogError(null),
    onSuccess: async () => {
      setCatalogPreview(null);
      setGithubUrl('');
      await publishQueryScopes(queryClient, ['catalogPack']);
    },
    onError: () => setCatalogError(t('general.catalog_assets.install_failed')),
  });
  const resetIdentityDismissalsMutation = useMutation<void, unknown>({
    mutationFn: () => commands.resetObjectIdentitySuggestionDismissals(activeGameId ?? ''),
    networkMode: 'always',
  });

  const gameCatalogCode = activeGame
    ? (['gimi', 'srmi', 'wwmi', 'zzmi', 'efmi'][activeGame.game_type] ?? 'game')
    : null;
  const githubSearchUrl = `https://github.com/search?q=${encodeURIComponent(
    gameCatalogCode
      ? `topic:emmm-catalog topic:emmm-game-${gameCatalogCode} archived:false`
      : 'topic:emmm-catalog archived:false',
  )}&type=repositories`;

  const openCatalogPackFolder = async () => {
    try {
      await commands.openCatalogPackFolder();
    } catch {
      setCatalogError(t('general.catalog_assets.invalid'));
    }
  };
  const isImportPending =
    previewGithubImportMutation.isPending ||
    previewLocalImportMutation.isPending ||
    installCatalogImportMutation.isPending;
  const catalogStatus =
    catalogPack?.state === 'ready'
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
              onClick={() => void openCatalogPackFolder()}
            >
              <FolderOpen size={15} /> {t('general.catalog_assets.open_folder')}
            </button>
            <button
              type="button"
              className="btn btn-sm btn-primary gap-2"
              disabled={refreshCatalogMutation.isPending}
              onClick={() => refreshCatalogMutation.mutate()}
            >
              <RefreshCw
                size={15}
                className={refreshCatalogMutation.isPending ? 'animate-spin' : undefined}
              />
              {t('general.catalog_assets.refresh')}
            </button>
          </div>
        }
      >
        <div className="space-y-4 border-b border-base-300/70 pb-4">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <div>
              <p className="text-sm font-medium">{t('general.catalog_assets.github_title')}</p>
              <p className="text-xs text-base-content/60">
                {t('general.catalog_assets.github_desc')}
              </p>
            </div>
            <a
              className="btn btn-sm btn-ghost gap-2"
              href={githubSearchUrl}
              rel="noreferrer"
              target="_blank"
            >
              <Search size={15} /> {t('general.catalog_assets.github_find')}
            </a>
          </div>
          <div className="flex flex-wrap gap-2">
            <input
              aria-label={t('general.catalog_assets.github_url_label')}
              className="input input-bordered input-sm min-w-64 flex-1"
              disabled={isImportPending}
              onChange={(event) => setGithubUrl(event.target.value)}
              placeholder={t('general.catalog_assets.github_url_placeholder')}
              type="url"
              value={githubUrl}
            />
            <button
              className="btn btn-sm btn-primary gap-2"
              disabled={!githubUrl.trim() || isImportPending}
              onClick={() => previewGithubImportMutation.mutate()}
              type="button"
            >
              <Download
                size={15}
                className={previewGithubImportMutation.isPending ? 'animate-pulse' : undefined}
              />
              {t('general.catalog_assets.github_check')}
            </button>
          </div>
          <p className="text-xs text-base-content/50">{t('general.catalog_assets.github_note')}</p>
          <div className="flex flex-wrap items-center justify-between gap-3 border-t border-base-300/70 pt-3">
            <div>
              <p className="text-sm font-medium">{t('general.catalog_assets.local_title')}</p>
              <p className="text-xs text-base-content/60">
                {t('general.catalog_assets.local_desc')}
              </p>
            </div>
            <button
              className="btn btn-sm btn-ghost gap-2"
              disabled={isImportPending}
              onClick={() => previewLocalImportMutation.mutate()}
              type="button"
            >
              <FileArchive
                size={15}
                className={previewLocalImportMutation.isPending ? 'animate-pulse' : undefined}
              />
              {t('general.catalog_assets.local_choose')}
            </button>
          </div>
        </div>
        <SettingsRow
          label={t('general.catalog_assets.reset_suggestions_title')}
          description={t('general.catalog_assets.reset_suggestions_desc')}
          control={
            <button
              className="btn btn-sm btn-ghost"
              disabled={!activeGameId || resetIdentityDismissalsMutation.isPending}
              onClick={() => resetIdentityDismissalsMutation.mutate()}
              type="button"
            >
              {t('general.catalog_assets.reset_suggestions_action')}
            </button>
          }
        />
        <div className="flex flex-wrap items-center justify-between gap-2 border-t border-base-300/70 pt-3">
          <div>
            <p className="text-sm font-medium">{t('general.catalog_assets.updates_title')}</p>
            <p className="text-xs text-base-content/60">
              {t('general.catalog_assets.updates_desc')}
            </p>
          </div>
          <div className="flex gap-2">
            <button
              type="button"
              className="btn btn-sm btn-ghost gap-2"
              disabled={
                checkCatalogUpdateMutation.isPending || installCatalogUpdateMutation.isPending
              }
              onClick={() => checkCatalogUpdateMutation.mutate()}
            >
              <RefreshCw
                size={15}
                className={checkCatalogUpdateMutation.isPending ? 'animate-spin' : undefined}
              />
              {t('general.catalog_assets.check_update')}
            </button>
            {catalogUpdate?.state === 'update_available' && (
              <button
                type="button"
                className="btn btn-sm btn-primary gap-2"
                disabled={installCatalogUpdateMutation.isPending}
                onClick={() => installCatalogUpdateMutation.mutate()}
              >
                <Download
                  size={15}
                  className={installCatalogUpdateMutation.isPending ? 'animate-pulse' : undefined}
                />
                {t('general.catalog_assets.install_update', {
                  version: catalogUpdate.available_version,
                })}
              </button>
            )}
          </div>
        </div>
        <p className="mt-3 text-xs text-base-content/60" aria-live="polite">
          {catalogError ||
            (catalogUpdate?.state === 'update_available'
              ? t('general.catalog_assets.update_available', {
                  version: catalogUpdate.available_version,
                })
              : catalogUpdate?.state === 'up_to_date'
                ? t('general.catalog_assets.up_to_date')
                : null)}
        </p>
      </SettingsSection>
      {catalogPreview && (
        <dialog
          aria-labelledby="catalog-import-review-title"
          className="modal modal-open"
          onClose={() => setCatalogPreview(null)}
          open
        >
          <div className="modal-box max-w-xl">
            <div className="flex items-start gap-3">
              <FileArchive className="mt-0.5 text-primary" size={20} />
              <div>
                <h2 id="catalog-import-review-title" className="text-lg font-semibold">
                  {t('general.catalog_assets.github_review_title')}
                </h2>
                <p className="mt-1 text-sm text-base-content/60">
                  {t('general.catalog_assets.review_source', {
                    source: catalogPreview.sourceLabel,
                  })}
                </p>
              </div>
            </div>
            <dl className="mt-5 grid grid-cols-[auto_1fr] gap-x-4 gap-y-2 text-sm">
              <dt className="text-base-content/60">{t('general.catalog_assets.github_version')}</dt>
              <dd>{catalogPreview.review.version}</dd>
              {catalogPreview.releaseTag && (
                <>
                  <dt className="text-base-content/60">
                    {t('general.catalog_assets.review_release')}
                  </dt>
                  <dd>{catalogPreview.releaseTag}</dd>
                </>
              )}
              <dt className="text-base-content/60">
                {t('general.catalog_assets.github_publisher')}
              </dt>
              <dd>{catalogPreview.review.publisher}</dd>
              <dt className="text-base-content/60">{t('general.catalog_assets.github_games')}</dt>
              <dd>{catalogPreview.review.supportedGames.join(', ')}</dd>
              <dt className="text-base-content/60">{t('general.catalog_assets.github_entries')}</dt>
              <dd>{catalogPreview.review.entries}</dd>
            </dl>
            {catalogPreview.replacesActivePack && (
              <p className="mt-4 rounded-box bg-warning/10 px-3 py-2 text-sm text-warning">
                {t('general.catalog_assets.github_replaces')}
              </p>
            )}
            <div className="modal-action">
              <button
                className="btn btn-ghost"
                disabled={installCatalogImportMutation.isPending}
                onClick={() => setCatalogPreview(null)}
                type="button"
              >
                {t('common:actions.cancel')}
              </button>
              <button
                className="btn btn-primary gap-2"
                disabled={installCatalogImportMutation.isPending}
                onClick={() => installCatalogImportMutation.mutate()}
                type="button"
              >
                <Download
                  size={15}
                  className={installCatalogImportMutation.isPending ? 'animate-pulse' : undefined}
                />
                {t('general.catalog_assets.github_install')}
              </button>
            </div>
          </div>
          <form className="modal-backdrop" method="dialog">
            <button type="button" onClick={() => setCatalogPreview(null)}>
              {t('common:actions.close')}
            </button>
          </form>
        </dialog>
      )}
    </div>
  );
}
