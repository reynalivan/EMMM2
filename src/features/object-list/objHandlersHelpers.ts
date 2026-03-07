/**
 * Shared helpers for useObjectListHandlers sub-hooks.
 * Pure functions — no React hooks.
 */

import { useAppStore } from '../../stores/useAppStore';
import type { MasterDbEntry } from './scanReviewHelpers';

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

import { invoke } from '@tauri-apps/api/core';
import type { QueryClient } from '@tanstack/react-query';
import { toast } from '../../stores/useToastStore';

/**
 * Executes the `import_mods_from_paths` command, invalidates the query cache,
 * and shows standard success/error toasts based on the move counts.
 */
export async function executeImportAndInvalidate(
  paths: string[],
  targetDir: string,
  queryClient: QueryClient,
  options: {
    isNewObject?: boolean;
    objectName?: string;
  },
): Promise<void> {
  const result = await invoke<{
    success: string[];
    failures: { path: string; error: string }[];
  }>('import_mods_from_paths', {
    paths,
    targetDir,
    strategy: 'Raw',
    dbJson: null,
  });

  queryClient.invalidateQueries({ queryKey: ['objects'] });
  queryClient.invalidateQueries({ queryKey: ['mod-folders'] });
  queryClient.invalidateQueries({ queryKey: ['category-counts'] });

  const movedCount = result.success.length;
  const failCount = result.failures.length;

  if (movedCount > 0) {
    const fails = failCount > 0 ? `, ${failCount} failed` : '';
    const label = options.isNewObject
      ? `Created ${options.objectName ?? 'Object'} with ${movedCount} item(s)${fails}`
      : `Moved ${movedCount} item(s)${options.objectName ? ` to ${options.objectName}` : ''}${fails}`;
    toast.success(label);
  } else if (failCount > 0) {
    const action = options.isNewObject
      ? 'Created Object but failed to move items'
      : 'Failed to move items';
    toast.error(`${action}: ${result.failures[0].error}`);
  }
}
