/** Shared game configuration types for the EMMM frontend. */

export enum GameType {
  GIMI = 0,
  SRMI = 1,
  WWMI = 2,
  ZZMI = 3,
  EFMI = 4,
}

/** Get the string identifier for a GameType (e.g. 0 -> "GIMI"). */
export function getGameTypeKey(type: GameType): string {
  return GameType[type];
}

export interface GameConfig {
  id: string;
  name: string;
  game_type: GameType;
  mod_path: string;
  game_exe: string;
  loader_exe: string | null;
  launch_args: string | null;
  warnings?: string[];
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

/** Lookup map: game_type → DaisyUI badge class. */
export const GAME_TYPE_COLORS: Record<number, string> = {
  [GameType.GIMI]: 'badge-primary',
  [GameType.SRMI]: 'badge-secondary',
  [GameType.WWMI]: 'badge-accent',
  [GameType.ZZMI]: 'badge-info',
  [GameType.EFMI]: 'badge-warning',
};
