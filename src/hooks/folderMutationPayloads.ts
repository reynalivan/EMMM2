/** Pure payload/patch derivations shared by the folder mutation hooks. */

import { toggleDisabledInPath } from '../lib/disabledPrefix';
import type { ModFolder, ModInfoUpdate } from '../types/mod';
import type { WorkspacePathRewrite } from '../types/workspace';

/** Cache patch for an info.json update — only the fields the update carries win. */
export function applyModInfoUpdate(folder: ModFolder, update: ModInfoUpdate): ModFolder {
  return {
    ...folder,
    is_favorite: update.is_favorite ?? folder.is_favorite,
    is_safe: update.is_safe ?? folder.is_safe,
    metadata: update.metadata ? { ...folder.metadata, ...update.metadata } : folder.metadata,
  };
}

/**
 * Path rewrites for a bulk toggle. The backend reports them when it can; older
 * results carry none, so they are reconstructed from the resulting paths and the
 * toggle direction.
 */
export function resolveTogglePathRewrites(
  successPaths: string[],
  reported: WorkspacePathRewrite[] | null | undefined,
  enable: boolean,
): WorkspacePathRewrite[] {
  if (reported && reported.length > 0) {
    return reported;
  }

  return successPaths.map((newPath) => ({
    old_path: toggleDisabledInPath(newPath, !enable),
    new_path: newPath,
  }));
}
