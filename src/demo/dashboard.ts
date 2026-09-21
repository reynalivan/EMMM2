import type { ActiveKeyBinding, DashboardPayload } from '@/shared/api/tauri/bindings.gen';

export const demoDashboardPayload: DashboardPayload = {
  stats: {
    total_games: 2,
    total_mods: 186,
    enabled_mods: 142,
    disabled_mods: 44,
    total_collections: 6,
  },
  duplicate_waste_bytes: 734_003_200,
  category_distribution: [
    { category: 'Characters', count: 62 },
    { category: 'Weapons', count: 48 },
    { category: 'Environment', count: 31 },
    { category: 'UI', count: 25 },
    { category: 'Other', count: 20 },
  ],
  game_distribution: [
    { game_id: 'demo-zenless', game_name: 'Zenless Zone Zero', count: 121 },
    { game_id: 'demo-stellar', game_name: 'Honkai: Star Rail', count: 65 },
  ],
  recent_mods: [
    {
      id: 'demo-recent-streetwear',
      game_id: 'demo-zenless',
      name: 'Nekomata Streetwear',
      game_name: 'Zenless Zone Zero',
      object_name: 'Nekomata',
      folder_path: 'Characters/Nekomata/Streetwear',
      indexed_at: '2026-09-13T06:42:00Z',
    },
    {
      id: 'demo-recent-lumina',
      game_id: 'demo-zenless',
      name: 'Lumina Square Recolor',
      game_name: 'Zenless Zone Zero',
      object_name: 'Lumina Square',
      folder_path: 'Environment/Lumina Square/Recolor',
      indexed_at: '2026-09-13T04:15:00Z',
    },
    {
      id: 'demo-recent-trailblazer',
      game_id: 'demo-stellar',
      name: 'Trailblazer Travel Set',
      game_name: 'Honkai: Star Rail',
      object_name: 'Trailblazer',
      folder_path: 'Characters/Trailblazer/Travel Set',
      indexed_at: '2026-09-12T16:30:00Z',
    },
    {
      id: 'demo-recent-interface',
      game_id: 'demo-zenless',
      name: 'Minimal HUD',
      game_name: 'Zenless Zone Zero',
      object_name: 'Interface',
      folder_path: 'UI/Interface/Minimal HUD',
      indexed_at: '2026-09-12T10:00:00Z',
    },
  ],
};

const demoKeybindingsByGame: Record<string, ActiveKeyBinding[]> = {
  'demo-zenless': [
    {
      mod_name: 'Nekomata Streetwear',
      folder_path: 'Characters/Nekomata/Streetwear',
      object_type: 'Character',
      matched_alias_name: 'Nekomata',
      section_name: 'Costume',
      key: 'F6',
      back: 'F7',
      control_kind: 'key_binding',
      value_summary: 'Streetwear / default',
    },
    {
      mod_name: 'Minimal HUD',
      folder_path: 'UI/Interface/Minimal HUD',
      object_type: 'UI',
      matched_alias_name: null,
      section_name: 'HUD preset',
      key: 'F8',
      back: null,
      control_kind: 'key_toggle',
      value_summary: 'Compact layout',
    },
    {
      mod_name: 'Lumina Square Recolor',
      folder_path: 'Environment/Lumina Square/Recolor',
      object_type: 'Environment',
      matched_alias_name: 'Lumina Square',
      section_name: 'Lighting',
      key: 'F9',
      back: 'F10',
      control_kind: 'key_binding',
      value_summary: 'Night / dusk',
    },
  ],
  'demo-stellar': [
    {
      mod_name: 'Trailblazer Travel Set',
      folder_path: 'Characters/Trailblazer/Travel Set',
      object_type: 'Character',
      matched_alias_name: 'Trailblazer',
      section_name: 'Outfit',
      key: 'F5',
      back: null,
      control_kind: 'key_toggle',
      value_summary: 'Travel set',
    },
    {
      mod_name: 'Astral Express Cabin',
      folder_path: 'Environment/Astral Express/Cabin',
      object_type: 'Environment',
      matched_alias_name: null,
      section_name: 'Cabin lighting',
      key: 'F11',
      back: 'F12',
      control_kind: 'key_binding',
      value_summary: 'Warm / neutral',
    },
  ],
};

export const demoDashboardGateway = {
  getDashboardStats: async (): Promise<DashboardPayload> => demoDashboardPayload,
  getActiveKeybindings: async (gameId: string): Promise<ActiveKeyBinding[]> =>
    demoKeybindingsByGame[gameId] ?? [],
};
