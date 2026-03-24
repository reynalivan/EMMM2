import React, { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useSettings } from '../../../hooks/useSettings';
import { Eye, EyeOff } from 'lucide-react';
import { useToastStore } from '../../../stores/useToastStore';

export default function AITab() {
  const { t } = useTranslation(['settings', 'common']);
  const { settings, updateAiConfig, isLoading } = useSettings();
  const { addToast } = useToastStore();
  const [showKey, setShowKey] = useState(false);

  // Local state for debouncing/cancel
  const [apiKey, setApiKey] = useState(settings?.ai.api_key || '');
  const [baseUrl, setBaseUrl] = useState(settings?.ai.base_url || '');

  // Synchronize local state when settings loads initially
  React.useEffect(() => {
    if (settings) {
      setApiKey(settings.ai.api_key || '');
      setBaseUrl(settings.ai.base_url || '');
    }
  }, [settings]);

  if (isLoading || !settings) {
    return <div className="p-4">{t('settings:ai.status.loading')}</div>;
  }

  const handleToggle = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const enabled = e.target.checked;
    try {
      await updateAiConfig.mutateAsync({ enabled });
      addToast(
        'success',
        enabled ? t('settings:ai.status.enabled') : t('settings:ai.status.disabled'),
      );
    } catch (err) {
      addToast(
        'error',
        t('settings:ai.status.update_failed', {
          error: err instanceof Error ? err.message : String(err),
        }),
      );
    }
  };

  const handleSave = async () => {
    try {
      await updateAiConfig.mutateAsync({ api_key: apiKey, base_url: baseUrl });
      addToast('success', t('settings:ai.status.saved'));
    } catch (err) {
      addToast(
        'error',
        t('settings:ai.status.save_failed', {
          error: err instanceof Error ? err.message : String(err),
        }),
      );
    }
  };

  return (
    <div className="space-y-6">
      <div className="card bg-base-200 shadow-sm border border-base-300">
        <div className="card-body">
          <h2 className="card-title text-xl text-primary flex items-center gap-2">
            {t('settings:ai.title')}
          </h2>
          <p className="mt-1 text-sm opacity-70">{t('settings:ai.desc')}</p>

          <div className="divider my-2"></div>

          <div className="form-control mb-4">
            <label className="label cursor-pointer justify-start gap-4">
              <input
                type="checkbox"
                className="toggle toggle-primary"
                checked={settings.ai.enabled}
                onChange={handleToggle}
              />
              <span className="label-text text-lg font-semibold">{t('settings:ai.enable')}</span>
            </label>
          </div>

          <div className="form-control w-full max-w-xl mb-4">
            <label className="label">
              <span className="label-text">{t('settings:ai.base_url')}</span>
              <span className="label-text-alt text-base-content/50">
                {t('settings:ai.base_url_desc')}
              </span>
            </label>
            <input
              type="text"
              placeholder={
                t('settings:ai.placeholder_url') || 'https://api.openai.com/v1/chat/completions'
              }
              className="input input-bordered w-full"
              value={baseUrl}
              onChange={(e) => setBaseUrl(e.target.value)}
            />
          </div>

          <div className="form-control w-full max-w-xl mb-6">
            <label className="label">
              <span className="label-text">{t('settings:ai.api_key')}</span>
            </label>
            <div className="join w-full">
              <input
                type={showKey ? 'text' : 'password'}
                placeholder={t('settings:ai.placeholder_key') || 'sk-...'}
                className="input input-bordered join-item w-full"
                value={apiKey}
                onChange={(e) => setApiKey(e.target.value)}
              />
              <button
                className="btn btn-square join-item"
                onClick={() => setShowKey(!showKey)}
                title={showKey ? t('settings:ai.hide_key') : t('settings:ai.show_key')}
              >
                {showKey ? <EyeOff className="w-5 h-5" /> : <Eye className="w-5 h-5" />}
              </button>
            </div>
            <label className="label">
              <span className="label-text-alt text-base-content/50">
                {t('settings:ai.api_key_desc')}
              </span>
            </label>
          </div>

          <div className="card-actions justify-end">
            <button className="btn btn-primary" onClick={handleSave}>
              {t('settings:ai.save')}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
