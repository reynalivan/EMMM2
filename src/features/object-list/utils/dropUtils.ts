/**
 * Drop zone rules for the ObjectList sidebar.
 *
 * The generic path classification lives in `lib/dropClassification` because the
 * shared `useFileDrop` hook and folder-grid need it too; only the zone rules
 * below are object-list's own.
 */

import { allUnsupported, type ClassifiedPaths } from '../../../lib/dropClassification';

export { classifyDroppedPaths, allUnsupported } from '../../../lib/dropClassification';
export type { ClassifiedPaths } from '../../../lib/dropClassification';

/** Supported drop zone types in the ObjectList sidebar */
export type DropZone = 'auto-organize' | 'item' | 'new-object';

/**
 * Validate whether a set of classified paths can be dropped onto a given zone.
 *
 * Rules:
 * - Any unsupported files → always blocked
 * - Archives → NOT allowed on 'new-object' zone
 * - Everything else → allowed on all zones
 */
export function validateDropForZone(
  zone: DropZone,
  classified: ClassifiedPaths,
): { valid: boolean; reason?: string } {
  if (allUnsupported(classified)) {
    return { valid: false, reason: 'Unsupported file type' };
  }

  if (zone === 'new-object' && classified.archives.length > 0) {
    return {
      valid: false,
      reason:
        'Archives cannot be added as new objects. Use Auto Organize or drop on a specific item.',
    };
  }

  return { valid: true };
}
