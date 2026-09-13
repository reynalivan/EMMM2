import { GameType } from '@/entities/game';

export interface ModViewerLaunchPolicy {
  visible: boolean;
  experimental: boolean;
}

const HIDDEN_POLICY: ModViewerLaunchPolicy = {
  visible: false,
  experimental: false,
};

/**
 * Keeps every Mod Viewer entry point on the same supported-game contract.
 */
export function getModViewerLaunchPolicy(
  executablePath: string | null | undefined,
  gameType: number | null | undefined,
): ModViewerLaunchPolicy {
  if (!executablePath) {
    return HIDDEN_POLICY;
  }

  if (gameType === GameType.GIMI || gameType === GameType.ZZMI || gameType === GameType.WWMI) {
    return { visible: true, experimental: false };
  }

  if (gameType === GameType.SRMI) {
    return { visible: true, experimental: true };
  }

  return HIDDEN_POLICY;
}
