import type { ComponentType } from 'react';
import { queryClient } from '@/shared/lib/queryClient';
import { useAppStore } from '@/app/store';
import { settingsKeys } from '@/entities/settings';
import { demoGameSettings } from './game';

export function getRootComponent(productionApp: ComponentType): ComponentType {
  // `useSettings` uses an infinite stale time. Hydrating its real query keeps
  // the app shell identical while avoiding a native request in browser-only QA.
  queryClient.setQueryData(settingsKeys.all, demoGameSettings);
  useAppStore.setState({ activeGameId: demoGameSettings.active_game_id });
  return productionApp;
}
