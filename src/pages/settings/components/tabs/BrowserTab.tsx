import { formatAppError } from '../../../../shared/lib/appError';
import { Globe, HardDrive, Trash2 } from 'lucide-react';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { commands } from '../../../../shared/api/tauri/bindings';
import { useToastStore } from '@/shared/ui/toast';
import { publishQueryScopes } from '@/shared/lib/queryRefresh';

const LEGACY_BROWSER_STORE_KEY = 'emmm-browser-store';

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
  const { addToast } = useToastStore();
  const [homepageDraft, setHomepageDraft] = useState('https://www.google.com');
  const [retentionDaysDraft, setRetentionDaysDraft] = useState('');

  const { data: homepageUrl } = useQuery({
    queryKey: ['browser_homepage'],
    queryFn: () => commands.browserGetHomepage(),
  });
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
    if (homepageUrl !== undefined) {
      setHomepageDraft(homepageUrl);
    }
  }, [homepageUrl]);

  useEffect(() => {
    if (retentionDays !== undefined) {
      setRetentionDaysDraft(String(retentionDays));
    }
  }, [retentionDays]);

  const setHomepageMutation = useMutation({
    mutationFn: (url: string) => commands.browserSetHomepage(url),
    onSuccess: async (_, url) => {
      queryClient.setQueryData(['browser_homepage'], url);
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
    const url = homepageDraft.trim();
    if (url && url !== homepageUrl) {
      setHomepageMutation.mutate(url);
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
    <div className="space-y-6 pb-12">
      {/* Browser Core */}
      <div className="card bg-base-200 shadow-sm border border-base-300">
        <div className="card-body">
          <h3 className="card-title text-lg flex items-center gap-2">
            <Globe size={20} className="text-info" />
            {t('settings:browser.title')}
          </h3>

          <div className="form-control w-full max-w-lg mt-2">
            <label className="label">
              <span className="label-text font-medium">{t('settings:browser.homepage')}</span>
            </label>
            <div className="flex gap-2">
              <input
                type="url"
                className="input input-bordered flex-1"
                value={homepageDraft}
                onChange={(event) => setHomepageDraft(event.target.value)}
                onBlur={saveHomepage}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    e.currentTarget.blur();
                  }
                }}
              />
              <button
                className="btn btn-outline"
                onClick={() => {
                  const defaultHomepage = 'https://www.google.com';
                  setHomepageDraft(defaultHomepage);
                  setHomepageMutation.mutate(defaultHomepage);
                }}
              >
                {t('settings:browser.reset')}
              </button>
            </div>
            <label className="label">
              <span className="label-text-alt text-base-content/60">
                {t('settings:browser.homepage_desc')}
              </span>
            </label>
          </div>
        </div>
      </div>

      {/* Storage & Retention */}
      <div className="card bg-base-200 shadow-sm border border-base-300">
        <div className="card-body">
          <h3 className="card-title text-lg flex items-center gap-2">
            <HardDrive size={20} className="text-warning" />
            {t('settings:browser.storage_title')}
          </h3>

          <div className="form-control w-full max-w-xs mt-4">
            <label className="label">
              <span className="label-text font-medium">{t('settings:browser.retention')}</span>
            </label>
            <div className="flex items-center gap-2">
              <input
                type="number"
                min="1"
                max="365"
                className="input input-bordered w-24"
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
            <label className="label">
              <span className="label-text-alt text-base-content/60">
                {t('settings:browser.retention_desc')}
              </span>
            </label>
          </div>

          <div className="divider opacity-30 my-2" />

          <div className="flex flex-wrap gap-3 mt-2">
            <button
              className="btn btn-outline btn-error gap-2"
              onClick={() => clearOldDownloadsMutation.mutate()}
              disabled={clearOldDownloadsMutation.isPending || retentionDays === undefined}
            >
              <Trash2 size={18} />
              {clearOldDownloadsMutation.isPending
                ? t('settings:browser.clearing')
                : t('settings:browser.clear_downloads')}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
