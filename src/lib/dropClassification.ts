/**
 * Extension-based classification of dropped paths.
 *
 * Lives here rather than in a feature because `hooks/useFileDrop` is shared by
 * every drop surface — folder-grid included — and must not inherit one
 * feature's opinion of what a valid drop is. Zone rules stay with their owner.
 */

const ARCHIVE_EXTENSIONS = new Set(['zip', '7z', 'rar']);
const IMAGE_EXTENSIONS = new Set(['png', 'jpg', 'jpeg', 'webp', 'gif']);
const INI_EXTENSIONS = new Set(['ini']);

/** Lowercased extension of a path, or '' when it has none. */
function getExtension(path: string): string {
  const lastDot = path.lastIndexOf('.');
  if (lastDot === -1) return '';
  // Handle paths with backslash/forward slash after the dot
  const afterDot = path.slice(lastDot + 1);
  if (afterDot.includes('\\') || afterDot.includes('/')) return '';
  return afterDot.toLowerCase();
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
