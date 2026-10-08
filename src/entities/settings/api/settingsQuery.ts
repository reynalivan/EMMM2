import { commands } from '@/shared/api/tauri/bindings';
import type { QueryClient } from '@tanstack/react-query';
import type { AppSettings } from '../model/settings';

export const settingsKeys = {
  all: ['settings'] as const,
};

export function publishSettingsSnapshot(client: QueryClient, snapshot: AppSettings): void {
  const current = client.getQueryData<AppSettings>(settingsKeys.all);
  if ((current?.revision ?? 0) > (snapshot.revision ?? 0)) return;
  void client
    .cancelQueries({ queryKey: settingsKeys.all }, { revert: false })
    .catch((error: unknown) => {
      console.error('Failed to cancel a superseded settings read', error);
    });
  client.setQueryData(settingsKeys.all, snapshot);
}

export const settingsQueryOptions = {
  queryKey: settingsKeys.all,
  queryFn: () => commands.getSettings(),
  staleTime: Infinity, // Settings don't change often from outside
};
