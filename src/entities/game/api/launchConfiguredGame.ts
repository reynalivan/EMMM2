import { commands } from '@/shared/api/tauri/bindings';

export async function launchConfiguredGame(
  gameId: string,
  closeAfterLaunch: boolean,
): Promise<void> {
  await commands.launchGame(gameId);

  if (closeAfterLaunch) {
    await commands.exitApp();
  }
}
