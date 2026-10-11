import type { ModPreviewImage } from '../../../src/shared/api/tauri/bindings.gen.js';
import { invokeInApp } from './ipc.js';

export async function listVisiblePreviewImages(input: {
  gameId: string;
  folderPath: string;
}): Promise<string[]> {
  const images = await invokeInApp<ModPreviewImage[]>(
    'list_mod_preview_images_with_visibility',
    input,
  );
  return images.filter((image) => !image.is_hidden).map((image) => image.path);
}
