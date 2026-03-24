import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { commands } from '../lib/bindings';
import type { AppSettings, AiConfig, PinVerifyStatus } from '../types/settings';
import type { GameConfig } from '../types/game';
import { useToastStore } from '../stores/useToastStore';
import { normalizeThemeSetting, type ThemeSetting } from '../features/settings/theme/themeOptions';
import i18n from '../lib/i18n';
import { useTranslation } from 'react-i18next';

// Re-export for consumers
export type { GameConfig, AppSettings, AiConfig, PinVerifyStatus };

export const settingsKeys = {
  all: ['settings'] as const,
};

export function useSettings() {
  const { t } = useTranslation(['settings', 'common', 'layout']);
  const queryClient = useQueryClient();
  const { addToast } = useToastStore();

  const settingsQuery = useQuery<AppSettings>({
    queryKey: settingsKeys.all,
    queryFn: () => commands.getSettings(),
    staleTime: Infinity, // Settings don't change often from outside
  });

  const saveSettingsMutation = useMutation({
    mutationFn: (newSettings: AppSettings) => commands.saveSettings({ settings: newSettings }),
    onSuccess: (_, newSettings) => {
      queryClient.setQueryData(settingsKeys.all, newSettings);
      addToast(
        'success',
        t('settings:toast.save_success', {
          defaultValue: 'Settings Saved: Configuration updated successfully.',
        }),
      );
    },
    onError: (err) => {
      console.error(err);
      addToast(
        'error',
        t('settings:toast.save_failed', {
          error: String(err),
          defaultValue: `Save Failed: ${String(err)}`,
        }),
      );
    },
  });

  const setPinMutation = useMutation({
    mutationFn: (pin: string) => commands.setPin({ pin, recoveryCode: undefined }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: settingsKeys.all });
      addToast(
        'success',
        t('settings:toast.pin_success', {
          defaultValue: 'PIN Updated: Safe Mode PIN has been set securely.',
        }),
      );
    },
    onError: (err) => {
      console.error(err);
      addToast(
        'error',
        t('settings:toast.pin_failed', {
          error: String(err),
          defaultValue: `PIN Update Failed: ${String(err)}`,
        }),
      );
    },
  });

  const setPinWithRecoveryMutation = useMutation({
    mutationFn: async (pin: string) => {
      // Generate a recovery code client-side, pass to backend for hashing
      const code = `EMMM-${crypto.randomUUID().slice(0, 4).toUpperCase()}-${crypto.randomUUID().slice(0, 4).toUpperCase()}-${crypto.randomUUID().slice(0, 4).toUpperCase()}`;
      await commands.setPin({ pin, recoveryCode: code });
      return code;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: settingsKeys.all });
    },
    onError: (err) => {
      console.error(err);
      addToast(
        'error',
        t('settings:toast.pin_failed', {
          error: String(err),
          defaultValue: `PIN Update Failed: ${String(err)}`,
        }),
      );
    },
  });

  const resetPinWithRecoveryMutation = useMutation({
    mutationFn: (code: string) => commands.resetPinWithRecoveryCode({ code }),
    onSuccess: (valid) => {
      if (valid) queryClient.invalidateQueries({ queryKey: settingsKeys.all });
    },
    onError: (err) => {
      console.error(err);
    },
  });

  const verifyPinMutation = useMutation({
    mutationFn: (pin: string) => commands.verifyPin({ pin }),
  });

  const maintenanceMutation = useMutation({
    mutationFn: () => commands.runMaintenance({}),
    onSuccess: (data) => {
      // data is [pruned, purged]
      const [pruned, purged] = data;
      addToast(
        'success',
        t('layout:maintenance.success', {
          pruned,
          purged,
          defaultValue: `Maintenance complete. Pruned ${pruned} thumbnails. Purged ${purged} old empty trash entries.`,
        }),
      );
    },
    onError: (err) => {
      addToast(
        'error',
        t('layout:maintenance.failed', {
          error: String(err),
          defaultValue: `Maintenance Failed: ${String(err)}`,
        }),
      );
    },
  });

  const aiConfigMutation = useMutation({
    mutationFn: async (newAiConfig: Partial<AiConfig>) => {
      if (!settingsQuery.data) throw new Error('Settings not loaded');
      const newSettings = {
        ...settingsQuery.data,
        ai: { ...settingsQuery.data.ai, ...newAiConfig },
      };
      return commands.saveSettings({ settings: newSettings });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: settingsKeys.all });
    },
    onError: (err) => {
      console.error(err);
      addToast(
        'error',
        t('settings:toast.ai_failed', {
          error: String(err),
          defaultValue: `Failed to update AI config: ${String(err)}`,
        }),
      );
    },
  });

  const updateThemeMutation = useMutation({
    mutationFn: async (theme: ThemeSetting) => {
      if (!settingsQuery.data) throw new Error('Settings not loaded');

      const newSettings = {
        ...settingsQuery.data,
        theme: normalizeThemeSetting(theme),
      };

      return commands.saveSettings({ settings: newSettings });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: settingsKeys.all });
      addToast(
        'success',
        t('settings:toast.theme_success', {
          defaultValue: 'Theme Updated: Appearance updated successfully.',
        }),
      );
    },
    onError: (err) => {
      console.error(err);
      addToast(
        'error',
        t('settings:toast.theme_failed', {
          error: String(err),
          defaultValue: `Theme Update Failed: ${String(err)}`,
        }),
      );
    },
  });

  return {
    settings: settingsQuery.data,
    isLoading: settingsQuery.isLoading,
    error: settingsQuery.error,
    saveSettings: saveSettingsMutation.mutate,
    saveSettingsAsync: saveSettingsMutation.mutateAsync,
    setPin: setPinMutation.mutate,
    setPinAsync: setPinMutation.mutateAsync,
    /** Sets PIN and returns plaintext recovery code (e.g. `EMMM-4F2A-9B87-CC1E`). Show once, never stored in plaintext. */
    setPinWithRecoveryAsync: setPinWithRecoveryMutation.mutateAsync,
    /** Validate recovery code and clear PIN. Returns true if code was valid. */
    resetPinWithRecoveryCodeAsync: resetPinWithRecoveryMutation.mutateAsync,
    verifyPin: verifyPinMutation.mutateAsync,
    runMaintenance: maintenanceMutation.mutate,
    updateAiConfig: aiConfigMutation,
    updateTheme: updateThemeMutation,
    updateLanguage: useMutation({
      mutationFn: async (language: string) => {
        if (!settingsQuery.data) throw new Error('Settings not loaded');
        const newSettings = {
          ...settingsQuery.data,
          language,
        };
        await commands.saveSettings({ settings: newSettings });
        await i18n.changeLanguage(language);
        return newSettings;
      },
      onSuccess: () => {
        queryClient.invalidateQueries({ queryKey: settingsKeys.all });
        addToast(
          'success',
          t('settings:toast.lang_success', {
            defaultValue: 'Language Updated: Interface language changed.',
          }),
        );
      },
      onError: (err) => {
        console.error(err);
        addToast(
          'error',
          t('settings:toast.lang_failed', {
            error: String(err),
            defaultValue: `Language Update Failed: ${String(err)}`,
          }),
        );
      },
    }),
  };
}
