import type { SaveSettingsResult } from '@/shared/api/tauri/bindings.gen';
import { demoGameSettings, setDemoSettings } from './game';
export function readDemoSettings(value: unknown) {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    return null;
  }

  const candidate = value as Record<string, unknown>;
  if (
    typeof candidate.theme !== 'string' ||
    typeof candidate.language !== 'string' ||
    !Array.isArray(candidate.games) ||
    typeof candidate.auto_close_launcher !== 'boolean' ||
    typeof candidate.ai !== 'object' ||
    candidate.ai === null ||
    typeof candidate.safety !== 'object' ||
    candidate.safety === null
  ) {
    return null;
  }

  return candidate as typeof demoGameSettings;
}

export function saveDemoSettings(settings: typeof demoGameSettings): SaveSettingsResult {
  return { settings: setDemoSettings(settings), sync_warning: null };
}
