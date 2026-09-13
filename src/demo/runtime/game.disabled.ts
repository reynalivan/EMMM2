import type { AppSettings } from '@/shared/api/tauri/bindings.gen';

function unavailableInAppMode(): never {
  throw new Error('Demo fixtures are unavailable outside the development demo server.');
}

export const demoGameGateway = {
  getSettings: async (): Promise<AppSettings> => unavailableInAppMode(),
};
