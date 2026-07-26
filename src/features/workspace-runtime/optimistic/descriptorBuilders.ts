import type { RuntimeEffectDescriptor, RuntimeRefreshEvent } from '../../../lib/runtimeEffects';
import { EMPTY_RUNTIME_EFFECT_DESCRIPTOR } from './descriptor';

export type RuntimeMutationClass =
  | 'workspaceOnly'
  | 'workspaceCorridor'
  | 'workspaceStructure'
  | 'folderStructureOnly'
  | 'folderConflictState'
  | 'folderMetadata'
  | 'folderMetadataPreview'
  | 'folderMetadataThumbnail'
  | 'folderSwitch'
  | 'objectRows'
  | 'objectStructure'
  | 'objectSwitch'
  | 'collectionsOnly'
  | 'collectionsCatalog'
  | 'collectionsState'
  | 'corridorState'
  | 'scannerConflictState'
  | 'scannerWorkspaceState'
  | 'trashOnly'
  | 'trashState'
  | 'thumbnailOnly'
  | 'previewOnly'
  | 'conflictsOnly'
  | 'dashboardKeybindings';

export interface WorkspacePathRewriteLike {
  old_path: string;
  new_path: string;
}

const runtimeMutationEvents: Record<RuntimeMutationClass, RuntimeRefreshEvent[]> = {
  workspaceOnly: ['workspaceChanged'],
  workspaceCorridor: ['workspaceChanged', 'corridorChanged'],
  workspaceStructure: ['workspaceChanged', 'folderStructureChanged'],
  folderStructureOnly: ['folderStructureChanged'],
  folderConflictState: ['conflictsChanged', 'corridorChanged'],
  folderMetadata: ['workspaceChanged'],
  folderMetadataPreview: ['folderMetadataChanged', 'previewChanged'],
  folderMetadataThumbnail: ['workspaceChanged', 'folderMetadataChanged', 'thumbnailChanged'],
  folderSwitch: [
    'workspaceChanged',
    'folderStructureChanged',
    'corridorChanged',
    'collectionsChanged',
    'dashboardChanged',
    'activeKeybindingsChanged',
    'previewChanged',
    'conflictsChanged',
  ],
  objectRows: ['workspaceChanged', 'objectRowsChanged'],
  objectStructure: ['workspaceChanged', 'objectRowsChanged', 'folderStructureChanged'],
  objectSwitch: [
    'workspaceChanged',
    'folderStructureChanged',
    'objectRowsChanged',
    'corridorChanged',
    'collectionsChanged',
    'dashboardChanged',
    'activeKeybindingsChanged',
    'previewChanged',
    'conflictsChanged',
  ],
  collectionsOnly: ['collectionsChanged'],
  collectionsCatalog: ['collectionsChanged', 'corridorChanged'],
  collectionsState: [
    'workspaceChanged',
    'folderStructureChanged',
    'corridorChanged',
    'collectionsChanged',
    'dashboardChanged',
    'activeKeybindingsChanged',
  ],
  corridorState: [
    'workspaceChanged',
    'folderStructureChanged',
    'corridorChanged',
    'collectionsChanged',
    'dashboardChanged',
    'activeKeybindingsChanged',
    'previewChanged',
    'conflictsChanged',
  ],
  scannerConflictState: ['workspaceChanged', 'folderStructureChanged', 'conflictsChanged'],
  scannerWorkspaceState: ['workspaceChanged', 'folderStructureChanged', 'corridorChanged'],
  trashOnly: ['trashChanged'],
  trashState: ['workspaceChanged', 'folderStructureChanged', 'trashChanged', 'corridorChanged'],
  thumbnailOnly: ['thumbnailChanged'],
  previewOnly: ['previewChanged'],
  conflictsOnly: ['conflictsChanged'],
  dashboardKeybindings: ['dashboardChanged', 'activeKeybindingsChanged'],
};

export function buildRefreshDescriptor(events: RuntimeRefreshEvent[]): RuntimeEffectDescriptor {
  return {
    ...EMPTY_RUNTIME_EFFECT_DESCRIPTOR,
    refreshEvents: events,
  };
}

/**
 * Refresh events plus one patched effect slice. The base descriptor has an empty
 * array in every non-refresh field, so spreading the patch replaces those wholesale.
 */
function withEffects(
  events: RuntimeRefreshEvent[],
  patch: Partial<RuntimeEffectDescriptor>,
): RuntimeEffectDescriptor {
  return { ...buildRefreshDescriptor(events), ...patch };
}

export function buildRuntimeMutationDescriptor(
  mutationClass: RuntimeMutationClass | RuntimeMutationClass[],
  extraEvents: RuntimeRefreshEvent[] = [],
): RuntimeEffectDescriptor {
  const classes = Array.isArray(mutationClass) ? mutationClass : [mutationClass];
  const events = classes.flatMap((entry) => runtimeMutationEvents[entry]);
  const uniqueEvents = [...new Set([...events, ...extraEvents])];
  return buildRefreshDescriptor(uniqueEvents);
}

export function buildPathRewriteDescriptor(
  oldPath: string,
  newPath: string,
  events: RuntimeRefreshEvent[],
): RuntimeEffectDescriptor {
  return withEffects(events, {
    rewrites: [{ oldPath, newPath }],
    thumbnailPaths: [oldPath],
  });
}

export function buildWorkspacePathRewritesDescriptor(
  rewrites: WorkspacePathRewriteLike[],
  events: RuntimeRefreshEvent[],
): RuntimeEffectDescriptor {
  return withEffects(events, {
    rewrites: rewrites.map((rewrite) => ({
      oldPath: rewrite.old_path,
      newPath: rewrite.new_path,
    })),
    thumbnailPaths: rewrites.map((rewrite) => rewrite.old_path),
  });
}

export function buildPathInvalidationDescriptor(
  path: string,
  events: RuntimeRefreshEvent[],
): RuntimeEffectDescriptor {
  return withEffects(events, {
    invalidatedPaths: [path],
    thumbnailPaths: [path],
  });
}

export function buildQueryInvalidationDescriptor(
  queryKeys: Array<readonly unknown[]>,
  events: RuntimeRefreshEvent[],
): RuntimeEffectDescriptor {
  return withEffects(events, { invalidatedQueryKeys: queryKeys });
}

export function buildQueryRemovalDescriptor(
  queryKeys: Array<readonly unknown[]>,
  events: RuntimeRefreshEvent[],
): RuntimeEffectDescriptor {
  return withEffects(events, { removedQueryKeys: queryKeys });
}
