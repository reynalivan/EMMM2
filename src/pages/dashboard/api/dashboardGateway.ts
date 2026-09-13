import type { ActiveKeyBinding, DashboardPayload } from '@/shared/api/tauri/bindings.gen';
import { commands } from '@/shared/api/tauri/bindings';
import { isDemoMode } from '@/shared/lib/appMode';
import { demoDashboardGateway } from '@/demo/dashboard';

interface DashboardGateway {
  getDashboardStats(): Promise<DashboardPayload>;
  getActiveKeybindings(gameId: string): Promise<ActiveKeyBinding[]>;
}

const tauriDashboardGateway: DashboardGateway = {
  getDashboardStats: () => commands.getDashboardStats(),
  getActiveKeybindings: (gameId) => commands.getActiveKeybindings(gameId),
};

export const dashboardGateway: DashboardGateway = isDemoMode
  ? demoDashboardGateway
  : tauriDashboardGateway;
