import type { HotkeyNotice } from '@/shared/api/tauri/bindings.gen';

export const hotkeyNoticeFixture = (patch: Partial<HotkeyNotice> = {}): HotkeyNotice => {
  const createdAt = Date.now();
  return {
    id: 'notice-1',
    sequence: 1,
    settings_revision: 0,
    game_id: 'game-a',
    game_name: 'Fixture game',
    mods_path: 'C:\\fixture\\Mods',
    root_identity: 'fixture-root',
    created_at_ms: createdAt,
    expires_at_ms: createdAt + 5000,
    kind: 'collection_applied',
    previous_collection: 'Exploration',
    collection: 'Combat',
    safe_mode: false,
    message: null,
    runtime_generation: 2,
    runtime_status: null,
    reload_binding: 'Ctrl+F10',
    hotkeys: {
      enabled: true,
      safe_mode: 'Alt+F5',
      next_preset: 'Ctrl+F5',
      prev_preset: 'Shift+F5',
      toggle_overlay: 'F7',
      preset_status_overlay_enabled: false,
      game_notification_overlay_enabled: false,
    },
    keyviewer_enabled: true,
    language: 'en',
    theme: 'onyx',
    ...patch,
  };
};
