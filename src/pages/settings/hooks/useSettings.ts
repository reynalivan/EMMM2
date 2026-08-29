import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { commands } from '../../../shared/api/tauri/bindings';
import type { AppSettings, AiConfig } from '@/pages/settings/model/settings';
import type { GameConfig } from '@/entities/game/model/game';
import { useToastStore } from '../../../app/store/useToastStore';
import { normalizeThemeSetting, type ThemeSetting } from '../../../shared/lib/themeOptions';
import i18n from '../../../shared/i18n/config';
import { useTranslation } from 'react-i18next';
import { publishQueryScopes } from '@/features/runtime-sync/queryRefresh';
import { settingsKeys, settingsQueryOptions } from './settingsQuery';
import { notifyCommittedMutationSyncWarning } from '../../../shared/lib/committedMutationWarning';

// Re-export for consumers
export type { GameConfig, AppSettings, AiConfig };

async function persistSettings(settings: AppSettings): Promise<AppSettings> {
  const result = await commands.saveSettings(settings);
  notifyCommittedMutationSyncWarning(result);
  return result.settings;
}

export function useSettings() {
  const { t } = useTranslation(['settings', 'common', 'layout']);
  const queryClient = useQueryClient();
  const { addToast } = useToastStore();

  const settingsQuery = useQuery<AppSettings>(settingsQueryOptions);

  const saveSettingsMutation = useMutation({
    mutationFn: persistSettings,
    onSuccess: async (savedSettings) => {
      const previousSettings = queryClient.getQueryData<AppSettings>(settingsKeys.all);
      queryClient.setQueryData(settingsKeys.all, savedSettings);
      const previousKeywords = previousSettings?.safety?.keywords ?? [];
      const savedKeywords = savedSettings.safety?.keywords ?? [];
      const keywordsChanged =
        previousKeywords.length !== savedKeywords.length ||
        previousKeywords.some((keyword, index) => keyword !== savedKeywords[index]);
      if (keywordsChanged) {
        await publishQueryScopes(queryClient, [
          'workspaceViewModel',
          'collections',
          'collectionRuntime',
        ]);
      }
      addToast('success', t('settings:toast.save_success'));
    },
    onError: (err) => {
      console.error(err);
      addToast(
        'error',
        t('settings:toast.save_failed', {
          error: String(err),
        }),
      );
    },
  });

  const maintenanceMutation = useMutation({
    mutationFn: () => commands.runMaintenance(),
    onSuccess: (data) => {
      addToast(
        'success',
        t('layout:maintenance.maintenance_success', {
          pruned: data,
        }),
      );
    },
    onError: (err) => {
      addToast(
        'error',
        t('layout:maintenance.failed', {
          error: String(err),
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
      return persistSettings(newSettings);
    },
    onSuccess: async () => {
      await publishQueryScopes(queryClient, ['settings']);
    },
    onError: (err) => {
      console.error(err);
      addToast(
        'error',
        t('settings:toast.ai_failed', {
          error: String(err),
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

      return persistSettings(newSettings);
    },
    onSuccess: async () => {
      await publishQueryScopes(queryClient, ['settings']);
      addToast('success', t('settings:toast.theme_success'));
    },
    onError: (err) => {
      console.error(err);
      addToast(
        'error',
        t('settings:toast.theme_failed', {
          error: String(err),
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
        await persistSettings(newSettings);
        await i18n.changeLanguage(language);
        return newSettings;
      },
      onSuccess: async () => {
        await publishQueryScopes(queryClient, ['settings']);
        addToast('success', t('settings:toast.lang_success'));
      },
      onError: (err) => {
        console.error(err);
        addToast(
          'error',
          t('settings:toast.lang_failed', {
            error: String(err),
          }),
        );
      },
    }),
  };
}
