import { formatAppError } from '../../../../shared/lib/appError';
import React, { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useSettings } from '@/entities/settings';
import { Eye, EyeOff } from 'lucide-react';
import { useToastStore } from '@/shared/ui/toast';
import { commands } from '../../../../shared/api/tauri/bindings';
import { SettingsRow, SettingsSection } from '../SettingsLayout';

type ApiKeyDraft = { value: string; revision: number; show: boolean };

export default function AITab() {
  const { t } = useTranslation(['settings', 'common']);
  const { settings, updateAiConfig, setAiApiKey, deleteAiApiKey, isLoading } = useSettings();
  const { addToast } = useToastStore();
  const [isTesting, setIsTesting] = useState(false);

  const [apiKeyDraft, setApiKeyDraft] = useState<ApiKeyDraft>({
    value: '',
    revision: 0,
    show: false,
  });
  const [baseUrlDraft, setBaseUrlDraft] = useState<{ value: string } | null>(null);
  const baseUrl = baseUrlDraft?.value ?? settings?.ai.base_url ?? '';

  const clearSubmittedKeyDraft = (revision: number) => {
    setApiKeyDraft((current) =>
      current.revision === revision
        ? { value: '', revision: current.revision + 1, show: false }
        : current,
    );
  };

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
    const submittedEndpointDraft = baseUrlDraft;
    const submittedKeyDraft = apiKeyDraft;
    try {
      await updateAiConfig.mutateAsync({ base_url: baseUrl });
      setBaseUrlDraft((current) => (current === submittedEndpointDraft ? null : current));
      if (submittedKeyDraft.value.trim()) {
        await setAiApiKey(submittedKeyDraft.value.trim());
        clearSubmittedKeyDraft(submittedKeyDraft.revision);
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
    const submittedKeyRevision = apiKeyDraft.revision;
    try {
      await deleteAiApiKey();
      clearSubmittedKeyDraft(submittedKeyRevision);
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
    <div>
      <SettingsSection
        id="ai-settings-heading"
        title={t('settings:ai.title')}
        description={t('settings:ai.desc')}
      >
        <SettingsRow
          label={t('settings:ai.enable')}
          control={
            <label className="label cursor-pointer gap-3 py-0">
              <input
                type="checkbox"
                aria-label={t('settings:ai.enable')}
                className="toggle toggle-primary toggle-sm"
                checked={settings.ai.enabled}
                onChange={handleToggle}
              />
            </label>
          }
        />
        <SettingsRow
          label={t('settings:ai.base_url')}
          description={t('settings:ai.base_url_desc')}
          control={
            <input
              type="text"
              aria-label={t('settings:ai.base_url')}
              placeholder={
                t('settings:ai.placeholder_url') || 'https://api.openai.com/v1/chat/completions'
              }
              className="input input-bordered input-sm w-full sm:w-96"
              value={baseUrl}
              onChange={(e) => setBaseUrlDraft({ value: e.target.value })}
            />
          }
        />
        <SettingsRow
          label={t('settings:ai.api_key')}
          description={
            settings.ai.has_api_key
              ? t('settings:ai.api_key_stored')
              : t('settings:ai.api_key_desc')
          }
          control={
            <div className="join w-full sm:w-96">
              <input
                type={apiKeyDraft.show ? 'text' : 'password'}
                aria-label={t('settings:ai.api_key')}
                placeholder={
                  settings.ai.has_api_key
                    ? t('settings:ai.stored_key_placeholder')
                    : t('settings:ai.status.placeholder_key') || 'sk-...'
                }
                className="input input-bordered input-sm join-item min-w-0 flex-1"
                value={apiKeyDraft.value}
                onChange={(e) => {
                  const value = e.target.value;
                  setApiKeyDraft((current) => ({
                    ...current,
                    value,
                    revision: current.revision + 1,
                  }));
                }}
              />
              <button
                className="btn btn-square btn-sm join-item"
                onClick={() => setApiKeyDraft((current) => ({ ...current, show: !current.show }))}
                title={apiKeyDraft.show ? t('settings:ai.hide_key') : t('settings:ai.show_key')}
              >
                {apiKeyDraft.show ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
              </button>
            </div>
          }
        />

        <div className="mt-3 flex flex-wrap justify-end gap-2 border-t border-base-300/70 pt-3">
          <button
            className="btn btn-outline btn-sm"
            onClick={handleTestConnection}
            disabled={!settings.ai.has_api_key || isTesting}
          >
            {isTesting ? t('settings:ai.testing') : t('settings:ai.test_connection')}
          </button>
          {settings.ai.has_api_key && (
            <button className="btn btn-ghost btn-sm text-error" onClick={handleRemoveKey}>
              {t('settings:ai.remove_key')}
            </button>
          )}
          <button className="btn btn-primary btn-sm" onClick={handleSave}>
            {t('settings:ai.save')}
          </button>
        </div>
      </SettingsSection>
    </div>
  );
}
