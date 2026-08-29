import { commands } from '../../shared/api/tauri/bindings';
import { useAppStore } from '../../app/store/useAppStore';
import type { ProcessedModInboxDestination } from './types';

export async function openProcessedDestinationInApp(
  destination: ProcessedModInboxDestination,
): Promise<void> {
  if (!destination.objectId) {
    throw new Error('Destination object no longer exists');
  }

  const object = await commands.getObject(destination.objectId);
  if (!object) {
    throw new Error(`Destination object no longer exists: ${destination.objectName}`);
  }

  const state = useAppStore.getState();
  state.setWorkspaceView('mods');
  state.setSelectedObjectFolderPath(object.folder_path);
  state.setGridSelection(new Set([destination.placedPath]));
}
