// Dummy Models directly for the Smart Demo Strip
// TODO: Replace with real models when wiring the real system later
export { usePrefersReducedMotion } from '../../hooks/usePrefersReducedMotion';

export type ModTypeTag = 'Character' | 'Weapon' | 'UI';

export interface DemoMod {
  id: string;
  name: string;
  typeTag: ModTypeTag;
  enabled: boolean;
}

export const DEMO_MODS: DemoMod[] = [
  { id: 'mod-1', name: 'Character Mod - Dress', typeTag: 'Character', enabled: true },
  { id: 'mod-2', name: 'Weapon Mod - Glowing', typeTag: 'Weapon', enabled: true },
  { id: 'mod-3', name: 'UI Mod - Transparent', typeTag: 'UI', enabled: true },
  { id: 'mod-4', name: 'Character Mod - Snow', typeTag: 'Character', enabled: false },
  { id: 'mod-5', name: 'Weapon Mod - Dark', typeTag: 'Weapon', enabled: false },
];

export interface DemoKeybind {
  keys: string;
  action: string;
}

export const DEMO_KEYBINDS: DemoKeybind[] = [
  { keys: '?', action: 'Show Keybinds' },
  { keys: 'Space', action: 'Toggle Preview' },
  { keys: 'Ctrl+K', action: 'Quick Search' },
];

export const SCENE_DURATION_MS = {
  A_AUTO_ORGANIZE: 5000,
  B_TOGGLE_PRESET: 4000,
  C_KEYBIND_SPOTLIGHT: 4500,
};
