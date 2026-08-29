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

/**
 * The real global hotkeys, with their shipped defaults (see the Rust
 * `HotkeyAction` variants). These fire while the GAME has focus, not the app —
 * which is the whole point of the scene, so the copy says so.
 */
export const DEMO_KEYBINDS: DemoKeybind[] = [
  { keys: 'F6', action: 'demo.action_next_preset' },
  { keys: 'F7', action: 'demo.action_toggle_overlay' },
  { keys: 'F8', action: 'demo.action_next_variant' },
];

export const SCENE_DURATION_MS = {
  A_AUTO_ORGANIZE: 5000,
  B_TOGGLE_PRESET: 4000,
  C_KEYBIND_SPOTLIGHT: 4500,
};
