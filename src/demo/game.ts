import type { AppSettings } from '@/shared/api/tauri/bindings.gen';

export const demoGameSettings: AppSettings = {
  theme: 'onyx',
  language: 'en',
  games: [
    {
      id: 'demo-zenless',
      name: 'Zenless Zone Zero',
      game_type: 3,
      mod_path: 'C:\\Demo\\Zenless\\Mods',
      game_exe: null,
      loader_exe: null,
      launch_args: null,
    },
    {
      id: 'demo-stellar',
      name: 'Honkai: Star Rail',
      game_type: 1,
      mod_path: 'C:\\Demo\\StarRail\\Mods',
      game_exe: null,
      loader_exe: null,
      launch_args: null,
    },
  ],
  active_game_id: 'demo-zenless',
  safety: { keywords: [] },
  ai: { enabled: false, has_api_key: false, base_url: null },
  auto_close_launcher: false,
};

let currentDemoSettings = demoGameSettings;

export function getDemoSettings(): AppSettings {
  return currentDemoSettings;
}

export function setDemoSettings(settings: AppSettings): AppSettings {
  currentDemoSettings = settings;
  return currentDemoSettings;
}

export const demoGameGateway = {
  getSettings: async (): Promise<AppSettings> => getDemoSettings(),
};
