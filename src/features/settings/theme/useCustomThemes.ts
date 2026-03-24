import { useState, useEffect } from 'react';
import { commands, type ThemeMetadata } from '../../../lib/bindings';

/**
 * useCustomThemes
 *
 * Fetches and manages the list of user-defined custom themes from the backend.
 */
export function useCustomThemes() {
  const [customThemes, setCustomThemes] = useState<ThemeMetadata[]>([]);
  const [loading, setLoading] = useState(true);

  const refreshCustomThemes = async () => {
    setLoading(true);
    try {
      const themes = await commands.listCustomThemes();
      setCustomThemes(themes);
    } catch (err) {
      console.error('Failed to list custom themes:', err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    refreshCustomThemes();
  }, []);

  return { customThemes, loading, refreshCustomThemes };
}
