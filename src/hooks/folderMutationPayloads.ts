/** Pure payload derivations shared by the folder mutation hooks. */

import { toggleDisabledInPath } from '../lib/disabledPrefix';
import type { WorkspacePathRewrite } from '../types/workspace';

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
