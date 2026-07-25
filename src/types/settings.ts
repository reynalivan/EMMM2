export type {
  ActiveKeyBinding,
  AiConfig,
  AppSettings,
  HotkeyConfig,
  KeyViewerConfig,
  SafeModeConfig,
} from '../lib/bindings.gen';

/** Frontend-only shape for PIN verification UI state (no Rust counterpart). */
export interface PinVerifyStatus {
  [key: string]: string | number | boolean | null | undefined;
}
