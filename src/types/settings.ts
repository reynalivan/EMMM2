import type { GameConfig } from './game';

export interface AiConfig {
  enabled: boolean;
  api_key: string | null;
  base_url: string | null;
}

export interface HotkeyConfig {
  enabled: boolean;
  cooldown_ms: number;
  toggle_safe_mode: string;
  next_preset: string;
  prev_preset: string;
  toggle_overlay: string;
  [key: string]: string | number | boolean | null | undefined;
}

export interface SafeModeConfig {
  enabled?: boolean;
  keywords: string[];
  [key: string]: string | number | boolean | string[] | null | undefined;
}

export interface AppSettings {
  ai: AiConfig;
  games: GameConfig[];
  safe_mode: SafeModeConfig;
  hotkeys: HotkeyConfig;
  keyviewer: KeyViewerConfig;
  theme: string;
  language: string;
  active_game_id?: string | null;
  auto_close_launcher?: boolean;
  [key: string]: unknown;
}

export interface ActiveKeyBinding {
  [key: string]: string | number | boolean | null | undefined;
}

export interface PinVerifyStatus {
  [key: string]: string | number | boolean | null | undefined;
}

export interface KeyViewerConfig {
  enabled: boolean;
  [key: string]: string | number | boolean | null | undefined;
}
