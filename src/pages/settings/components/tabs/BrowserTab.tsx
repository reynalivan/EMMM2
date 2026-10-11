import { formatAppError } from '../../../../shared/lib/appError';
import { Trash2 } from 'lucide-react';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { commands } from '../../../../shared/api/tauri/bindings';
import { useToastStore } from '@/shared/ui/toast';
import { useAppStore } from '@/app/store';
import { publishQueryScopes } from '@/shared/lib/queryRefresh';
import { SettingsRow, SettingsSection } from '../SettingsLayout';

const LEGACY_BROWSER_STORE_KEY = 'emmm-browser-store';
const DEFAULT_HOMEPAGE = 'https://www.google.com';

type HomepageDraft = { gameId: string; value: string };
type HomepageSave = { gameId: string; url: string; draft: HomepageDraft };

function legacyRetentionDays(): number | null {
  try {
    const raw = localStorage.getItem(LEGACY_BROWSER_STORE_KEY);
    if (!raw) return null;

    const parsed: unknown = JSON.parse(raw);
    if (!parsed || typeof parsed !== 'object' || !('state' in parsed)) return null;

    const state = parsed.state;
    if (!state || typeof state !== 'object' || !('retentionDays' in state)) return null;

    const days = state.retentionDays;
    return typeof days === 'number' && Number.isInteger(days) && days >= 1 && days <= 365
      ? days
      : null;
  } catch (error) {
    console.warn('Could not read the legacy browser settings.', error);
    return null;
  }
}

function removeLegacyBrowserStore(): void {
  try {
    localStorage.removeItem(LEGACY_BROWSER_STORE_KEY);
  } catch (error) {
    console.warn('Could not remove the legacy browser settings.', error);
  }
}

export default function BrowserTab() {
  const { t } = useTranslation(['settings', 'common']);
  const queryClient = useQueryClient();
  const activeGameId = useAppStore((state) => state.activeGameId);
  const { addToast } = useToastStore();
  const [homepageDraft, setHomepageDraft] = useState<HomepageDraft | null>(null);
  const [retentionDaysDraft, setRetentionDaysDraft] = useState('');

  const homepageQuery = useQuery({
    queryKey: ['browser_homepage', activeGameId],
    queryFn: () => commands.browserGetHomepage(activeGameId!),
    enabled: Boolean(activeGameId),
  });
  const homepageUrl = homepageQuery.data;
  const homepageReady =
    Boolean(activeGameId) && homepageQuery.isSuccess && !homepageQuery.isFetching;
  const activeHomepageDraft = homepageDraft?.gameId === activeGameId ? homepageDraft : null;
  const { data: retentionDays } = useQuery({
    queryKey: ['browser-retention-days'],
    queryFn: async () => {
      const legacyDays = legacyRetentionDays();
      const days = await commands.browserGetRetentionDays(legacyDays);
      removeLegacyBrowserStore();
      return days;
    },
  });

  useEffect(() => {
    if (retentionDays !== undefined) {
      setRetentionDaysDraft(String(retentionDays));
    }
  }, [retentionDays]);

  const setHomepageMutation = useMutation({
    mutationFn: ({ gameId, url }: HomepageSave) => commands.browserSetHomepage(gameId, url),
    onSuccess: async (_, { gameId, url, draft }) => {
      const queryKey = ['browser_homepage', gameId];
      await queryClient.cancelQueries({ queryKey, exact: true });
      queryClient.setQueryData(queryKey, url);
      setHomepageDraft((current) => (current === draft ? null : current));
      await publishQueryScopes(queryClient, ['browserHomepage']);
      addToast('success', t('settings:browser.homepage_success'));
    },
    onError: (err) => {
      addToast('error', t('settings:browser.homepage_failed', { error: formatAppError(err) }));
    },
  });

  const setRetentionDaysMutation = useMutation({
    mutationFn: (days: number) => commands.browserSetRetentionDays(days),
    onSuccess: (_result, days) => {
      queryClient.setQueryData(['browser-retention-days'], days);
      setRetentionDaysDraft(String(days));
      addToast('success', t('settings:browser.retention_success'));
    },
    onError: (err) => {
      addToast('error', t('settings:browser.retention_failed', { error: formatAppError(err) }));
    },
  });

  const clearOldDownloadsMutation = useMutation({
    mutationFn: () => commands.browserClearOldDownloads(),
    onSuccess: async (count) => {
      addToast('success', t('settings:browser.clear_success', { count }));
      await publishQueryScopes(queryClient, ['browserDownloads']);
    },
    onError: (err) => {
      addToast('error', t('settings:browser.clear_failed', { error: formatAppError(err) }));
    },
  });

  const saveHomepage = () => {
    if (!activeGameId || !homepageReady || !activeHomepageDraft) return;
    const url = activeHomepageDraft.value.trim();
    if (url && url !== homepageUrl) {
      setHomepageMutation.mutate({ gameId: activeGameId, url, draft: activeHomepageDraft });
    }
  };

  const saveRetentionDays = () => {
    const days = Number(retentionDaysDraft);
    if (!Number.isInteger(days) || days < 1 || days > 365) {
      setRetentionDaysDraft(retentionDays === undefined ? '' : String(retentionDays));
      addToast('error', t('settings:browser.retention_invalid'));
      return;
    }

    if (days !== retentionDays) {
      setRetentionDaysMutation.mutate(days);
    }
  };

  return (
    <div>
      <SettingsSection id="browser-settings-heading" title={t('settings:browser.title')}>
        <SettingsRow
          label={t('settings:browser.homepage')}
          description={t('settings:browser.homepage_desc')}
          control={
            <div className="flex w-full gap-2 sm:w-96">
              <input
                type="url"
                aria-label={t('settings:browser.homepage')}
                className="input input-bordered input-sm min-w-0 flex-1"
                value={activeHomepageDraft?.value ?? homepageUrl ?? ''}
                onChange={(event) => {
                  if (activeGameId && homepageReady) {
                    setHomepageDraft({ gameId: activeGameId, value: event.target.value });
                  }
                }}
                onBlur={saveHomepage}
                disabled={!homepageReady}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    e.currentTarget.blur();
                  }
                }}
              />
              <button
                className="btn btn-outline btn-sm"
                disabled={!homepageReady}
                onClick={() => {
                  if (!activeGameId || !homepageReady) return;
                  const draft = { gameId: activeGameId, value: DEFAULT_HOMEPAGE };
                  setHomepageDraft(draft);
                  setHomepageMutation.mutate({
                    gameId: activeGameId,
                    url: DEFAULT_HOMEPAGE,
                    draft,
                  });
                }}
              >
                {t('settings:browser.reset')}
              </button>
            </div>
          }
        />
      </SettingsSection>

      <SettingsSection id="browser-storage-heading" title={t('settings:browser.storage_title')}>
        <SettingsRow
          label={t('settings:browser.retention')}
          description={t('settings:browser.retention_desc')}
          control={
            <div className="flex items-center gap-2">
              <input
                type="number"
                aria-label={t('settings:browser.retention')}
                min="1"
                max="365"
                className="input input-bordered input-sm w-20"
                value={retentionDaysDraft}
                onChange={(event) => setRetentionDaysDraft(event.target.value)}
                onBlur={saveRetentionDays}
                onKeyDown={(event) => {
                  if (event.key === 'Enter') {
                    event.currentTarget.blur();
                  }
                }}
                disabled={setRetentionDaysMutation.isPending}
              />
              <span className="text-sm text-base-content/70">{t('settings:browser.days')}</span>
            </div>
          }
        />
        <div className="flex justify-end border-t border-base-300/70 pt-3">
          <button
            className="btn btn-ghost btn-sm gap-2 text-error hover:bg-error/10"
            onClick={() => clearOldDownloadsMutation.mutate()}
            disabled={clearOldDownloadsMutation.isPending || retentionDays === undefined}
          >
            <Trash2 size={15} />
            {clearOldDownloadsMutation.isPending
              ? t('settings:browser.clearing')
              : t('settings:browser.clear_downloads')}
          </button>
        </div>
      </SettingsSection>
    </div>
  );
}
