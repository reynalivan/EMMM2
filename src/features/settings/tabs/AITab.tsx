import React, { useState } from 'react';
import { useSettings } from '../../../hooks/useSettings';
import { Eye, EyeOff } from 'lucide-react';
import { useToastStore } from '../../../stores/useToastStore';

export default function AITab() {
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
    return <div className="p-4">Loading AI config...</div>;
  }

  const handleToggle = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const enabled = e.target.checked;
    try {
      await updateAiConfig.mutateAsync({ enabled });
      addToast('success', `AI Reranking ${enabled ? 'enabled' : 'disabled'}`);
    } catch (err) {
      addToast(
        'error',
        `Failed to update toggle: ${err instanceof Error ? err.message : String(err)}`,
      );
    }
  };

  const handleSave = async () => {
    try {
      await updateAiConfig.mutateAsync({ api_key: apiKey, base_url: baseUrl });
      addToast('success', 'AI configuration saved');
    } catch (err) {
      addToast(
        'error',
        `Failed to save AI config: ${err instanceof Error ? err.message : String(err)}`,
      );
    }
  };

  return (
    <div className="space-y-6">
      <div className="card bg-base-200 shadow-sm border border-base-300">
        <div className="card-body">
          <h2 className="card-title text-xl text-primary flex items-center gap-2">
            AI Intelligent Reranking
          </h2>
          <p className="text-sm text-base-content/70">
            When enabled, EMMM2 will use an LLM via the provided OpenAI-compatible endpoint to
            attempt recovery of mod folders that were marked as "Needs Review" by the local scoring
            pipeline. This provides significantly better accuracy for misnamed folders at the cost
            of some additional latency and external API requests.
          </p>

          <div className="divider my-2"></div>

          <div className="form-control mb-4">
            <label className="label cursor-pointer justify-start gap-4">
              <input
                type="checkbox"
                className="toggle toggle-primary"
                checked={settings.ai.enabled}
                onChange={handleToggle}
              />
              <span className="label-text text-lg font-semibold">Enable AI Reranking</span>
            </label>
          </div>

          <div className="form-control w-full max-w-xl mb-4">
            <label className="label">
              <span className="label-text">API Provider Base URL</span>
              <span className="label-text-alt text-base-content/50">
                (Must be an OpenAI-compatible /v1/chat/completions endpoint)
              </span>
            </label>
            <input
              type="text"
              placeholder="https://api.openai.com/v1/chat/completions"
              className="input input-bordered w-full"
              value={baseUrl}
              onChange={(e) => setBaseUrl(e.target.value)}
            />
          </div>

          <div className="form-control w-full max-w-xl mb-6">
            <label className="label">
              <span className="label-text">AI API Key</span>
            </label>
            <div className="join w-full">
              <input
                type={showKey ? 'text' : 'password'}
                placeholder="sk-..."
                className="input input-bordered join-item w-full"
                value={apiKey}
                onChange={(e) => setApiKey(e.target.value)}
              />
              <button
                className="btn btn-square join-item"
                onClick={() => setShowKey(!showKey)}
                title={showKey ? 'Hide Key' : 'Show Key'}
              >
                {showKey ? <EyeOff className="w-5 h-5" /> : <Eye className="w-5 h-5" />}
              </button>
            </div>
            <label className="label">
              <span className="label-text-alt text-base-content/50">
                This key is stored locally and sent only to the configured base URL.
              </span>
            </label>
          </div>

          <div className="card-actions justify-end">
            <button className="btn btn-primary" onClick={handleSave}>
              Save Config
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
