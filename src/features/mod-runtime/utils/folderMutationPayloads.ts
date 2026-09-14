/** Pure payload derivations shared by the folder mutation hooks. */

import { toggleDisabledInPath } from '@/shared/lib/disabledPrefix';
import type { WorkspacePathRewrite } from '@/entities/workspace';

/**
 * Path rewrites for a bulk toggle. The backend reports them when it can; older
 * results omit the field, so only those use the legacy reconstruction. An empty
 * reported list means no folders were renamed.
 */
export function resolveTogglePathRewrites(
  successPaths: string[],
  reported: WorkspacePathRewrite[] | null | undefined,
  enable: boolean,
): WorkspacePathRewrite[] {
  if (reported != null) {
    return reported;
  }

  return successPaths.map((newPath) => ({
    old_path: toggleDisabledInPath(newPath, !enable),
    new_path: newPath,
  }));
}
