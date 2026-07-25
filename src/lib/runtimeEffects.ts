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

export interface ObjectCountDeltaEffect {
  objectId: string;
  delta: number;
}

export interface RuntimeEffectDescriptor {
  rewrites: Array<{ oldPath: string; newPath: string }>;
  invalidatedPaths: string[];
  objectCountDeltas: ObjectCountDeltaEffect[];
  thumbnailPaths: string[];
  removedQueryKeys: Array<readonly unknown[]>;
  invalidatedQueryKeys: Array<readonly unknown[]>;
  refreshEvents: RuntimeRefreshEvent[];
}
