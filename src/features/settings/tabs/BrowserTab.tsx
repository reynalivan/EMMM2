import { Globe, HardDrive, ShieldAlert, Trash2 } from 'lucide-react';
import { useBrowserStore } from '../../../stores/useBrowserStore';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { useToastStore } from '../../../stores/useToastStore';

export default function BrowserTab() {
  const {
    autoImport,
    setAutoImport,
    skipGamePicker,
    setSkipGamePicker,
    allowedExtensions,
    setAllowedExtensions,
    retentionDays,
    setRetentionDays,
    downloadsRoot,
    setDownloadsRoot,
  } = useBrowserStore();

  const queryClient = useQueryClient();
  const { addToast } = useToastStore();

  // Homepage URL from backend
  const { data: homepageUrl = 'https://www.google.com' } = useQuery({
    queryKey: ['browser_homepage'],
    queryFn: () => invoke<string>('browser_get_homepage'),
  });

  const setHomepageMutation = useMutation({
    mutationFn: (url: string) => invoke('browser_set_homepage', { url }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['browser_homepage'] });
      addToast('success', 'Homepage updated successfully');
    },
    onError: (err) => {
      addToast('error', `Failed to update homepage: ${String(err)}`);
    },
  });

  const clearOldDownloadsMutation = useMutation({
    mutationFn: () => invoke<number>('browser_clear_old_downloads'),
    onSuccess: (count) => {
      addToast('success', `Cleared ${count} old downloads.`);
      // Invalidate downloads query if it happens to be active
      queryClient.invalidateQueries({ queryKey: ['browser-downloads'] });
    },
    onError: (err) => {
      addToast('error', `Failed to clear downloads: ${String(err)}`);
    },
  });

  const handleExtensionsChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const exts = e.target.value
      .split(',')
      .map((s) => s.trim())
      .filter((s) => s.length > 0);
    setAllowedExtensions(exts);
  };

  return (
    <div className="space-y-6 pb-12">
      {/* Browser Core */}
      <div className="card bg-base-200 shadow-sm border border-base-300">
        <div className="card-body">
          <h3 className="card-title text-lg flex items-center gap-2">
            <Globe size={20} className="text-info" />
            Discover & Browser
          </h3>

          <div className="form-control w-full max-w-lg mt-2">
            <label className="label">
              <span className="label-text font-medium">Homepage URL</span>
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
                Reset
              </button>
            </div>
            <label className="label">
              <span className="label-text-alt text-base-content/60">
                The default page opened when you click Discover (must start with http/https).
              </span>
            </label>
          </div>
        </div>
      </div>

      {/* Import Behavior */}
      <div className="card bg-base-200 shadow-sm border border-base-300">
        <div className="card-body">
          <h3 className="card-title text-lg flex items-center gap-2">
            <ShieldAlert size={20} className="text-success" />
            Smart Import Behavior
          </h3>

          <div className="form-control max-w-md mt-2">
            <label className="label cursor-pointer justify-start gap-4">
              <input
                type="checkbox"
                className="toggle toggle-success"
                checked={autoImport}
                onChange={(e) => setAutoImport(e.target.checked)}
              />
              <span className="label-text font-medium">Enable Automatic Analysis</span>
            </label>
            <p className="text-sm text-base-content/60 mt-1 pl-13">
              When a mod finishes downloading, automatically extract and deep-match it. If disabled,
              imports must be started manually.
            </p>
          </div>

          <div className="form-control max-w-md mt-4">
            <label className="label cursor-pointer justify-start gap-4">
              <input
                type="checkbox"
                className="toggle toggle-primary"
                checked={skipGamePicker}
                onChange={(e) => setSkipGamePicker(e.target.checked)}
              />
              <span className="label-text font-medium">Skip Game Picker (Single Game)</span>
            </label>
            <p className="text-sm text-base-content/60 mt-1 pl-13">
              If you only have one game configured, skip the Game Picker modal and import instantly.
            </p>
          </div>

          <div className="form-control w-full max-w-sm mt-4">
            <label className="label">
              <span className="label-text font-medium">Allowed Extensions</span>
            </label>
            <input
              type="text"
              className="input input-bordered w-full"
              value={allowedExtensions.join(', ')}
              onChange={handleExtensionsChange}
              placeholder=".zip, .7z, .rar"
            />
            <label className="label">
              <span className="label-text-alt text-base-content/60">
                Comma-separated list of extensions the browser is allowed to download.
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
            Storage & Retention
          </h3>

          <div className="form-control w-full max-w-lg mt-2">
            <label className="label">
              <span className="label-text font-medium">Custom Downloads Directory</span>
            </label>
            <input
              type="text"
              className="input input-bordered w-full"
              value={downloadsRoot}
              onChange={(e) => setDownloadsRoot(e.target.value)}
              placeholder="Default: AppData/EMMM2/BrowserDownloads"
            />
            <label className="label">
              <span className="label-text-alt text-base-content/60">
                Leave empty to use the default app data location. Restart required if changed.
              </span>
            </label>
          </div>

          <div className="form-control w-full max-w-xs mt-4">
            <label className="label">
              <span className="label-text font-medium">Retention Days</span>
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
              <span className="text-sm text-base-content/70">days</span>
            </div>
            <label className="label">
              <span className="label-text-alt text-base-content/60">
                How long to keep downloaded archives before automatic cleanup.
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
              {clearOldDownloadsMutation.isPending ? 'Clearing...' : 'Clear Old Downloads Now'}
            </button>
            <button
              className="btn btn-outline gap-2"
              onClick={() => {
                // Future: implement invoke('browser_clear_cache') if available
                addToast('info', 'Browser cache clearing is not implemented yet.');
              }}
            >
              Clear Browser Cache
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
