import { useState } from 'react';
import { X } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useSettings } from '@/entities/settings';
import { useToastStore } from '@/shared/ui/toast';
import { formatAppError } from '../../../../shared/lib/appError';
import { SettingsSection } from '../SettingsLayout';

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
    <div>
      <SettingsSection
        id="privacy-settings-heading"
        title={t('settings:privacy.title')}
        description={t('settings:privacy.desc')}
      />

      <SettingsSection id="privacy-keywords-heading" title={t('settings:privacy.keywords_title')}>
        <div className="mb-3 flex items-center gap-2">
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
            <span key={keyword} className="badge badge-ghost h-7 gap-1 border-base-300 pl-3 pr-1.5">
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
      </SettingsSection>
    </div>
  );
}
