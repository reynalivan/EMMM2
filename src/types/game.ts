/** Shared game configuration types for the EMMM2 frontend. */

/** Represents a configured 3DMigoto game instance. */
export interface GameConfig {
  id: string;
  name: string;
  game_type: string;
  path: string;
  mods_path: string;
  launcher_path: string;
  launch_args: string | null;
}

/**
 * Single source of truth for game types.
 * Keep in sync with Rust `XXMI_TARGETS` in `src-tauri/src/services/validator.rs`.
 */
export const GAME_OPTIONS = [
  { value: 'GIMI', label: 'Genshin Impact (GIMI)', badgeColor: 'badge-primary' },
  { value: 'SRMI', label: 'Honkai Star Rail (SRMI)', badgeColor: 'badge-secondary' },
  { value: 'WWMI', label: 'Wuthering Waves (WWMI)', badgeColor: 'badge-accent' },
  { value: 'ZZMI', label: 'Zenless Zone Zero (ZZMI)', badgeColor: 'badge-info' },
  { value: 'EFMI', label: 'Arknight Endfield (EFMI)', badgeColor: 'badge-warning' },
] as const;

/** Derived lookup map: game_type → DaisyUI badge class. */
export const GAME_TYPE_COLORS: Record<string, string> = Object.fromEntries(
  GAME_OPTIONS.map((opt) => [opt.value, opt.badgeColor]),
);
