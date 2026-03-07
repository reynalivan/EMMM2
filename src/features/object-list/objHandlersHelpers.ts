/**
 * Shared helpers for useObjectListHandlers sub-hooks.
 * Pure functions — no React hooks.
 */

import { useAppStore } from '../../stores/useAppStore';
import type { MasterDbEntry } from './ScanReviewModal';

/** Sync explorer navigation after a folder rename on disk. */
export function syncExplorerAfterRename(modPath: string, oldPath: string, newPath: string): void {
  const { explorerSubPath, setExplorerSubPath, setCurrentPath } = useAppStore.getState();
  if (!explorerSubPath) return;

  const clean = (p: string) => p.replace(/\\/g, '/');
  const cleanMod = clean(modPath);
  const cleanOld = clean(oldPath);
  const cleanNew = clean(newPath);
  const currentAbs = `${cleanMod}/${clean(explorerSubPath)}`;

  if (currentAbs === cleanOld || currentAbs.startsWith(cleanOld + '/')) {
    const updated =
      currentAbs === cleanOld ? cleanNew : currentAbs.replace(cleanOld + '/', cleanNew + '/');
    let sub = updated.substring(cleanMod.length);
    if (sub.startsWith('/')) sub = sub.substring(1);
    if (sub && sub !== explorerSubPath) {
      setExplorerSubPath(sub);
      setCurrentPath(sub.split('/'));
    }
  }
}

/** Parse MasterDB JSON string into typed entries. Safely returns [] on failure. */
export function parseMasterDb(dbJson: string): MasterDbEntry[] {
  try {
    const parsed = JSON.parse(dbJson);
    if (!Array.isArray(parsed)) return [];
    return parsed.map((e: Record<string, unknown>) => ({
      name: String(e.name ?? ''),
      object_type: String(e.object_type ?? 'Other'),
      tags: Array.isArray(e.tags) ? (e.tags as string[]) : [],
      metadata: (e.metadata as Record<string, unknown>) ?? null,
      thumbnail_path: e.thumbnail_path ? String(e.thumbnail_path) : null,
    }));
  } catch {
    return [];
  }
}
