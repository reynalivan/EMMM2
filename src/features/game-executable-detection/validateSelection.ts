import { commands } from '@/shared/api/tauri/bindings';
import type { GameConfig } from '@/entities/game';

export async function validateSelectedGameExecutables(
  games: ReadonlyArray<Pick<GameConfig, 'id' | 'game_type' | 'game_exe'>>,
  selectedPaths: ReadonlyMap<string, string>,
): Promise<void> {
  for (const game of games) {
    if (game.game_exe && selectedPaths.get(game.id) === game.game_exe) {
      await commands.validateGameExecutable({
        game_type: game.game_type,
        executable_path: game.game_exe,
      });
    }
  }
}
