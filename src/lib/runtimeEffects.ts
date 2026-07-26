// Shared runtime effect contracts. Leaf module: imported by both the
// runtime-sync and workspace-runtime features, imports neither.
export type RuntimeRefreshEvent =
  | 'workspaceChanged'
  | 'objectRowsChanged'
  | 'folderStructureChanged'
  | 'folderMetadataChanged'
  | 'previewChanged'
  | 'thumbnailChanged'
  | 'conflictsChanged'
  | 'corridorChanged'
  | 'collectionsChanged'
  | 'dashboardChanged'
  | 'activeKeybindingsChanged'
  | 'trashChanged'
  | 'settingsChanged'
  | 'browserDownloadsChanged'
  | 'browserImportQueueChanged'
  | 'browserHomepageChanged'
  | 'dedupChanged'
  | 'dedupReportChanged'
  | 'scannerChanged'
  | 'pinsChanged';

/**
 * Invalidation-only contract: effects either evict caches (`thumbnailPaths`,
 * `removedQueryKeys`), update store-side selection state (`rewrites`,
 * `invalidatedPaths`), or schedule refetches (`refreshEvents`,
 * `invalidatedQueryKeys`). Nothing here writes domain data into the query
 * cache — fresh data always comes from a refetch.
 */
export interface RuntimeEffectDescriptor {
  rewrites: Array<{ oldPath: string; newPath: string }>;
  invalidatedPaths: string[];
  thumbnailPaths: string[];
  removedQueryKeys: Array<readonly unknown[]>;
  invalidatedQueryKeys: Array<readonly unknown[]>;
  refreshEvents: RuntimeRefreshEvent[];
}
