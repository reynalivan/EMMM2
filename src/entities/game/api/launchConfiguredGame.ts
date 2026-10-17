import { commands } from '@/shared/api/tauri/bindings';
import type { GameConfig } from '../model/game';

type LaunchConfiguration = Pick<GameConfig, 'game_exe' | 'launch_mode'>;

/**
 * XXMI starts the selected importer through its own launcher. A standalone
 * configuration, by contrast, cannot safely launch without its game binary.
 */
export function requiresGameExecutable(game: LaunchConfiguration): boolean {
  return game.launch_mode !== 'xxmi_managed' && !game.game_exe?.trim();
}

export async function launchConfiguredGame(
  gameId: string,
  closeAfterLaunch: boolean,
): Promise<void> {
  await commands.launchGame(gameId);

  if (closeAfterLaunch) {
    await commands.exitApp();
  }
}
