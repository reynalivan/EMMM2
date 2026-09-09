import { formatAppError } from '../../../../shared/lib/appError';
import React, { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useSettings } from '../../hooks/useSettings';
import { Eye, EyeOff } from 'lucide-react';
import { useToastStore } from '@/shared/ui/toast';
import { commands } from '../../../../shared/api/tauri/bindings';

export default function AITab() {
  const { t } = useTranslation(['settings', 'common']);
  const { settings, updateAiConfig, setAiApiKey, deleteAiApiKey, isLoading } = useSettings();
  const { addToast } = useToastStore();
  const [showKey, setShowKey] = useState(false);
  const [isTesting, setIsTesting] = useState(false);

  // Local state for debouncing/cancel
  const [apiKey, setApiKey] = useState('');
  const [apiKeyDirty, setApiKeyDirty] = useState(false);
  const [baseUrl, setBaseUrl] = useState(settings?.ai.base_url || '');

  // Synchronize local state when settings loads initially
  React.useEffect(() => {
    if (settings) {
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
          error: formatAppError(err),
        }),
      );
    }
  };

  const handleSave = async () => {
    try {
      await updateAiConfig.mutateAsync({ base_url: baseUrl });
      if (apiKeyDirty && apiKey.trim()) {
        await setAiApiKey(apiKey.trim());
        setApiKey('');
        setApiKeyDirty(false);
        setShowKey(false);
      }
      addToast('success', t('settings:ai.status.saved'));
    } catch (err) {
      addToast(
        'error',
        t('settings:ai.status.save_failed', {
          error: formatAppError(err),
        }),
      );
    }
  };

  const handleRemoveKey = async () => {
    try {
      await deleteAiApiKey();
      setApiKey('');
      setApiKeyDirty(false);
      setShowKey(false);
      addToast('success', t('settings:ai.status.key_removed'));
    } catch (err) {
      addToast(
        'error',
        t('settings:ai.status.delete_failed', {
          error: formatAppError(err),
        }),
      );
    }
  };

  const handleTestConnection = async () => {
    setIsTesting(true);
    try {
      await commands.testAiConnection();
      addToast('success', t('settings:ai.status.test_success'));
    } catch (err) {
      addToast(
        'error',
        t('settings:ai.status.test_failed', {
          error: formatAppError(err),
        }),
      );
    } finally {
      setIsTesting(false);
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
                placeholder={
                  settings.ai.has_api_key
                    ? t('settings:ai.stored_key_placeholder')
                    : t('settings:ai.status.placeholder_key') || 'sk-...'
                }
                className="input input-bordered join-item w-full"
                value={apiKey}
                onChange={(e) => {
                  setApiKey(e.target.value);
                  setApiKeyDirty(true);
                }}
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
                {settings.ai.has_api_key
                  ? t('settings:ai.api_key_stored')
                  : t('settings:ai.api_key_desc')}
              </span>
            </label>
          </div>

          <div className="card-actions justify-end">
            <button
              className="btn btn-outline"
              onClick={handleTestConnection}
              disabled={!settings.ai.has_api_key || isTesting}
            >
              {isTesting ? t('settings:ai.testing') : t('settings:ai.test_connection')}
            </button>
            {settings.ai.has_api_key && (
              <button className="btn btn-ghost text-error" onClick={handleRemoveKey}>
                {t('settings:ai.remove_key')}
              </button>
            )}
            <button className="btn btn-primary" onClick={handleSave}>
              {t('settings:ai.save')}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
