/**
 * Demo data for the WelcomeScreen animation strip.
 * These are intentionally standalone types and constants, not runtime hook barrels.
 */

export type ModTypeTag = 'Character' | 'Weapon' | 'UI';

export interface DemoMod {
  id: string;
  name: string;
  typeTag: ModTypeTag;
  enabled: boolean;
}

export const DEMO_MODS: DemoMod[] = [
  { id: 'mod-1', name: 'demo.mod_character_dress', typeTag: 'Character', enabled: true },
  { id: 'mod-2', name: 'demo.mod_weapon_glowing', typeTag: 'Weapon', enabled: true },
  { id: 'mod-3', name: 'demo.mod_ui_transparent', typeTag: 'UI', enabled: true },
  { id: 'mod-4', name: 'demo.mod_character_snow', typeTag: 'Character', enabled: false },
  { id: 'mod-5', name: 'demo.mod_weapon_dark', typeTag: 'Weapon', enabled: false },
];

export interface DemoKeybind {
  keys: string;
  action: string;
}

export const DEMO_KEYBINDS: DemoKeybind[] = [
  { keys: '?', action: 'demo.action_show_keybinds' },
  { keys: 'Space', action: 'demo.action_toggle_preview' },
  { keys: 'Ctrl+K', action: 'demo.action_quick_search' },
];

export const SCENE_DURATION_MS = {
  A_AUTO_ORGANIZE: 5000,
  B_TOGGLE_PRESET: 4000,
  C_KEYBIND_SPOTLIGHT: 4500,
};
