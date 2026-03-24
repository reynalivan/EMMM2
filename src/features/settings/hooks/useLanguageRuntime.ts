import { useEffect } from 'react';
import { useSettings } from '../../../hooks/useSettings';
import i18n from '../../../lib/i18n';

export function useLanguageRuntime() {
  const { settings } = useSettings();

  useEffect(() => {
    if (settings?.language && i18n.language !== settings.language) {
      i18n.changeLanguage(settings.language).catch(console.error);
    }
  }, [settings?.language]);
}
