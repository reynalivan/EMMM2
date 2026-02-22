/**
 * Drop zone utilities — file type classification and zone validation.
 * Used by DnD components to determine valid drop targets and classify incoming files.
 */

/** Supported drop zone types in the ObjectList sidebar */
export type DropZone = 'auto-organize' | 'item' | 'new-object';

const ARCHIVE_EXTENSIONS = new Set(['zip', '7z', 'rar']);
const IMAGE_EXTENSIONS = new Set(['png', 'jpg', 'jpeg', 'webp', 'gif']);
const INI_EXTENSIONS = new Set(['ini']);

/** Get lowercased extension from a path string */
function getExtension(path: string): string {
  const lastDot = path.lastIndexOf('.');
  if (lastDot === -1) return '';
  // Handle paths with backslash/forward slash after the dot
  const afterDot = path.slice(lastDot + 1);
  if (afterDot.includes('\\') || afterDot.includes('/')) return '';
  return afterDot.toLowerCase();
}

export function isArchivePath(path: string): boolean {
  return ARCHIVE_EXTENSIONS.has(getExtension(path));
}

export function isImagePath(path: string): boolean {
  return IMAGE_EXTENSIONS.has(getExtension(path));
}

export function isIniPath(path: string): boolean {
  return INI_EXTENSIONS.has(getExtension(path));
}

/** Result of classifying dropped file paths */
export interface ClassifiedPaths {
  folders: string[];
  archives: string[];
  iniFiles: string[];
  images: string[];
  unsupported: string[];
}

/**
 * Classify dropped paths by extension.
 * Paths without an extension are assumed to be folders.
 */
export function classifyDroppedPaths(paths: string[]): ClassifiedPaths {
  const result: ClassifiedPaths = {
    folders: [],
    archives: [],
    iniFiles: [],
    images: [],
    unsupported: [],
  };

  for (const p of paths) {
    const ext = getExtension(p);
    if (ext === '') {
      // No extension → treat as folder
      result.folders.push(p);
    } else if (ARCHIVE_EXTENSIONS.has(ext)) {
      result.archives.push(p);
    } else if (INI_EXTENSIONS.has(ext)) {
      result.iniFiles.push(p);
    } else if (IMAGE_EXTENSIONS.has(ext)) {
      result.images.push(p);
    } else {
      result.unsupported.push(p);
    }
  }
  return result;
}

/** Check if any paths have unsupported file types */
export function hasUnsupported(classified: ClassifiedPaths): boolean {
  return classified.unsupported.length > 0;
}

/** Check if ALL paths are unsupported */
export function allUnsupported(classified: ClassifiedPaths): boolean {
  return (
    classified.unsupported.length > 0 &&
    classified.folders.length === 0 &&
    classified.archives.length === 0 &&
    classified.iniFiles.length === 0 &&
    classified.images.length === 0
  );
}

/** Check if there are any archive files */
export function hasArchives(classified: ClassifiedPaths): boolean {
  return classified.archives.length > 0;
}

/** Check if there are ONLY archive files (no folders/ini/images) */
export function onlyArchives(classified: ClassifiedPaths): boolean {
  return (
    classified.archives.length > 0 &&
    classified.folders.length === 0 &&
    classified.iniFiles.length === 0 &&
    classified.images.length === 0
  );
}

/** Total count of supported items */
export function supportedCount(classified: ClassifiedPaths): number {
  return (
    classified.folders.length +
    classified.archives.length +
    classified.iniFiles.length +
    classified.images.length
  );
}

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

  if (zone === 'new-object' && hasArchives(classified)) {
    return {
      valid: false,
      reason:
        'Archives cannot be added as new objects. Use Auto Organize or drop on a specific item.',
    };
  }

  return { valid: true };
}
