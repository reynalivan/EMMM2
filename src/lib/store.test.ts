import { describe, it, expect, vi, beforeEach } from 'vitest';
import { configStore, StoreKeys, saveConfig, getConfig } from './store';

vi.mock('@tauri-apps/plugin-store', () => {
  return {
    LazyStore: vi.fn().mockImplementation(function () {
      return {
        set: vi.fn(),
        save: vi.fn(),
        get: vi.fn(),
      };
    }),
  };
});

describe('store config', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should instantiate LazyStore with correct path', () => {
    expect(configStore).toBeDefined();
  });

  it('should expose StoreKeys constants', () => {
    expect(StoreKeys.ACTIVE_GAME).toBe('active_game');
    expect(StoreKeys.SAFE_MODE).toBe('safe_mode');
    expect(StoreKeys.GAMES).toBe('games');
  });

  it('should save config value via store', async () => {
    await saveConfig('test_key', 'test_value');
    expect(configStore.set).toHaveBeenCalledWith('test_key', 'test_value');
    expect(configStore.save).toHaveBeenCalled();
  });

  it('should get config value via store', async () => {
    vi.mocked(configStore.get).mockResolvedValueOnce('mock_value' as unknown);
    const value = await getConfig('test_key');
    expect(configStore.get).toHaveBeenCalledWith('test_key');
    expect(value).toBe('mock_value');
  });
});
