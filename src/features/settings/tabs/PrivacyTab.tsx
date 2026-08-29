import { useState } from 'react';
import { Shield, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useSettings } from '../../../hooks/useSettings';
import { useToastStore } from '../../../stores/useToastStore';
import { formatAppError } from '../../../lib/appError';

function normalizeKeywords(values: string[]): string[] {
  return [...new Set(values.map((value) => value.trim().toLowerCase()).filter(Boolean))];
}

export default function PrivacyTab() {
  const { t } = useTranslation(['settings', 'common']);
  const { settings, saveSettingsAsync } = useSettings();
  const { addToast } = useToastStore();
  const [keywordInput, setKeywordInput] = useState('');

  if (!settings) return <div>{t('common:status.loading')}</div>;

  const keywords = normalizeKeywords(settings.safety.keywords);
  const saveKeywords = async (nextKeywords: string[]) => {
    try {
      await saveSettingsAsync({
        ...settings,
        safety: { keywords: normalizeKeywords(nextKeywords) },
      });
    } catch (error) {
      addToast(
        'error',
        t('settings:privacy.keywords_update_failed', { error: formatAppError(error) }),
      );
    }
  };

  const addKeyword = async () => {
    const keyword = keywordInput.trim().toLowerCase();
    if (!keyword) return;
    if (keywords.includes(keyword)) {
      addToast('warning', t('settings:privacy.keywords_exists'));
      return;
    }
    setKeywordInput('');
    await saveKeywords([...keywords, keyword]);
  };

  return (
    <div className="card bg-base-200 shadow-sm border border-base-300">
      <div className="card-body">
        <div className="flex gap-4">
          <div className="p-3 rounded-xl bg-primary/10 text-primary">
            <Shield size={24} />
          </div>
          <div>
            <h3 className="card-title text-lg">{t('settings:privacy.title')}</h3>
            <p className="text-sm opacity-70 max-w-md mt-1">{t('settings:privacy.desc')}</p>
          </div>
        </div>

        <div className="mt-6 pt-6 border-t border-base-300">
          <h4 className="text-sm font-bold uppercase tracking-wider opacity-50 mb-3">
            {t('settings:privacy.keywords_title')}
          </h4>
          <div className="flex items-center gap-2 mb-3">
            <input
              type="text"
              className="input input-sm input-bordered flex-1"
              placeholder={t('settings:privacy.keywords_placeholder')}
              value={keywordInput}
              onChange={(event) => setKeywordInput(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === 'Enter') {
                  event.preventDefault();
                  void addKeyword();
                }
              }}
            />
            <button
              type="button"
              className="btn btn-sm btn-primary"
              onClick={() => void addKeyword()}
            >
              {t('settings:privacy.keywords_add')}
            </button>
          </div>
          <div className="flex flex-wrap gap-2">
            {keywords.map((keyword) => (
              <span key={keyword} className="badge badge-neutral gap-1 pl-3 pr-2 py-3">
                {keyword}
                <button
                  type="button"
                  className="btn btn-ghost btn-xs btn-circle"
                  aria-label={t('settings:privacy.keywords_remove', { keyword })}
                  onClick={() => void saveKeywords(keywords.filter((value) => value !== keyword))}
                >
                  <X size={12} />
                </button>
              </span>
            ))}
            {keywords.length === 0 && (
              <span className="text-xs opacity-60">{t('settings:privacy.keywords_empty')}</span>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
