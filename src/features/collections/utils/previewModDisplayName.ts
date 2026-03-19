import type { CollectionPreviewMod } from '../../../types/collection';

function getPathSegments(folderPath: string): string[] {
  return folderPath.replace(/\\/g, '/').replace(/\/+$/g, '').split('/').filter(Boolean);
}

export function getPreviewModDisplayName(mod: CollectionPreviewMod): string {
  const fallbackName = mod.actual_name?.trim() || 'Unknown Mod';
  if (!mod.id.startsWith('nested_')) {
    return fallbackName;
  }

  const segments = getPathSegments(mod.folder_path);
  if (segments.length < 2) {
    return fallbackName;
  }

  const parentName = segments[segments.length - 2]?.trim();
  if (!parentName || parentName === fallbackName) {
    return fallbackName;
  }

  return `${parentName} > ${fallbackName}`;
}
