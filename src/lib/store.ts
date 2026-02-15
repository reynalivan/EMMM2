import { LazyStore } from '@tauri-apps/plugin-store';

const STORE_PATH = 'config.json';
export const configStore = new LazyStore(STORE_PATH);

export const StoreKeys = {
  ACTIVE_GAME: 'active_game',
  SAFE_MODE: 'safe_mode',
  GAMES: 'games',
} as const;

export async function saveConfig(key: string, value: unknown) {
  await configStore.set(key, value);
  await configStore.save();
}

export async function getConfig<T>(key: string): Promise<T | null | undefined> {
  return await configStore.get<T>(key);
}
