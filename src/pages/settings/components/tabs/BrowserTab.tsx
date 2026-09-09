import { formatAppError } from '../../../../shared/lib/appError';
import { Globe, HardDrive, Trash2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useBrowserStore } from '@/entities/browser';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { commands } from '../../../../shared/api/tauri/bindings';
import { useToastStore } from '@/shared/ui/toast';
import { publishQueryScopes } from '@/shared/lib/queryRefresh';

export default function BrowserTab() {
  const { t } = useTranslation(['settings', 'common']);
  const { retentionDays, setRetentionDays } = useBrowserStore();

  const queryClient = useQueryClient();
  const { addToast } = useToastStore();

  // Homepage URL from backend
  const { data: homepageUrl = 'https://www.google.com' } = useQuery({
    queryKey: ['browser_homepage'],
    queryFn: () => commands.browserGetHomepage(),
  });

  const setHomepageMutation = useMutation({
    mutationFn: (url: string) => commands.browserSetHomepage(url),
    onSuccess: async () => {
      await publishQueryScopes(queryClient, ['browserHomepage']);
      addToast('success', t('settings:browser.homepage_success'));
    },
    onError: (err) => {
      addToast('error', t('settings:browser.homepage_failed', { error: formatAppError(err) }));
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
                value={homepageUrl}
                onChange={() => {
                  // Optimistic local update isn't strictly necessary,
                  // but we handle submit on blur or enter for simplicity
                }}
                onBlur={(e) => setHomepageMutation.mutate(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    setHomepageMutation.mutate(e.currentTarget.value);
                    e.currentTarget.blur();
                  }
                }}
              />
              <button
                className="btn btn-outline"
                onClick={() => setHomepageMutation.mutate('https://www.google.com')}
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
                value={retentionDays}
                onChange={(e) => setRetentionDays(parseInt(e.target.value) || 30)}
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
              disabled={clearOldDownloadsMutation.isPending}
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
