import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { useToastStore } from '../stores/useToastStore';

import type { GameConfig } from '../types/game';

// Re-export for consumers
export type { GameConfig };

export interface SafeModeConfig {
  enabled: boolean;
  pin_hash: string | null;
  keywords: string[];
  force_exclusive_mode: boolean;
}

export interface PinVerifyStatus {
  valid: boolean;
  attempts_remaining: number;
  locked_seconds_remaining: number;
}

export interface AppSettings {
  theme: string;
  language: string;
  games: GameConfig[];
  active_game_id: string | null;
  safe_mode: SafeModeConfig;
}

export const settingsKeys = {
  all: ['settings'] as const,
};

export function useSettings() {
  const queryClient = useQueryClient();
  const { addToast } = useToastStore();

  const settingsQuery = useQuery<AppSettings>({
    queryKey: settingsKeys.all,
    queryFn: () => invoke<AppSettings>('get_settings'),
    staleTime: Infinity, // Settings don't change often from outside
  });

  const saveSettingsMutation = useMutation({
    mutationFn: (newSettings: AppSettings) => invoke('save_settings', { settings: newSettings }),
    onSuccess: (_, newSettings) => {
      queryClient.setQueryData(settingsKeys.all, newSettings);
      addToast('success', 'Settings Saved: Configuration updated successfully.');
    },
    onError: (err) => {
      console.error(err);
      addToast('error', `Save Failed: ${String(err)}`);
    },
  });

  const setPinMutation = useMutation({
    mutationFn: (pin: string) => invoke('set_safe_mode_pin', { pin }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: settingsKeys.all });
      addToast('success', 'PIN Updated: Safe Mode PIN has been set securely.');
    },
    onError: (err) => {
      console.error(err);
      addToast('error', `PIN Update Failed: ${String(err)}`);
    },
  });

  const verifyPinMutation = useMutation({
    mutationFn: (pin: string) => invoke<PinVerifyStatus>('verify_pin', { pin }),
  });

  const maintenanceMutation = useMutation({
    mutationFn: () => invoke<string>('run_maintenance'),
    onSuccess: (message) => {
      addToast('success', message);
    },
    onError: (err) => {
      addToast('error', `Maintenance Failed: ${String(err)}`);
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
    verifyPin: verifyPinMutation.mutateAsync,
    runMaintenance: maintenanceMutation.mutate,
  };
}
