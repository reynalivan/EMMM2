import { useQuery } from '@tanstack/react-query';
import { commands } from '../lib/bindings';
import type { AppSettings } from '../types/settings';

export const settingsKeys = {
  all: ['settings'] as const,
};

export const settingsQueryOptions = {
  queryKey: settingsKeys.all,
  queryFn: () => commands.getSettings(),
  staleTime: Infinity, // Settings don't change often from outside
};

/**
 * Authoritative Safe Mode flag, read straight from the settings query.
 * Never mirror this in the app store — a mirror goes stale the moment
 * Safe Mode is toggled in Settings.
 *
 * Defaults to `true` while settings are loading so nothing renders unmasked
 * before the real value lands.
 */
export function useSafeMode(): boolean {
  const { data } = useQuery<AppSettings>(settingsQueryOptions);
  return data?.safe_mode.enabled ?? true;
}
