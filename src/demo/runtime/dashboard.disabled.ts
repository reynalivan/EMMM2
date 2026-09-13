import type { ActiveKeyBinding, DashboardPayload } from '@/shared/api/tauri/bindings.gen';

function unavailableInAppMode(): never {
  throw new Error('Demo fixtures are unavailable outside the development demo server.');
}

export const demoDashboardGateway = {
  getDashboardStats: async (): Promise<DashboardPayload> => unavailableInAppMode(),
  getActiveKeybindings: async (_gameId: string): Promise<ActiveKeyBinding[]> =>
    unavailableInAppMode(),
};
