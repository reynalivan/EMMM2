import { trace, info, error, attachConsole } from '@tauri-apps/plugin-log';

export async function initLogger() {
  await attachConsole();
}

export const logger = {
  trace,
  info,
  error,
};
