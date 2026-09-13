import type { AppSettings } from '@/shared/api/tauri/bindings.gen';
import { commands } from '@/shared/api/tauri/bindings';
import { isDemoMode } from '@/shared/lib/appMode';
import { demoGameGateway } from '@/demo/game';

interface GameGateway {
  getSettings(): Promise<AppSettings>;
}

const tauriGameGateway: GameGateway = {
  getSettings: () => commands.getSettings(),
};

export const gameGateway: GameGateway = isDemoMode ? demoGameGateway : tauriGameGateway;
