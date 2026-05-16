import type { RuntimeRefreshEvent } from '../../runtime-sync/queryRefresh';
import {
  EMPTY_RUNTIME_EFFECT_DESCRIPTOR,
  mergeRuntimeEffectDescriptors,
  type RuntimeEffectDescriptor,
} from './descriptor';

export type RuntimeMutationClass =
  | 'workspaceOnly'
  | 'workspaceCorridor'
  | 'workspaceStructure'
  | 'folderStructureOnly'
  | 'folderConflictState'
  | 'folderMetadata'
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

function buildRefreshDescriptor(events: RuntimeRefreshEvent[]): RuntimeEffectDescriptor {
  return {
    ...EMPTY_RUNTIME_EFFECT_DESCRIPTOR,
    refreshEvents: events,
  };
}

export function buildRuntimeRefreshDescriptor(
  events: RuntimeRefreshEvent[],
): RuntimeEffectDescriptor {
  return buildRefreshDescriptor(events);
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
  return mergeRuntimeEffectDescriptors(buildRefreshDescriptor(events), {
    ...EMPTY_RUNTIME_EFFECT_DESCRIPTOR,
    rewrites: [{ oldPath, newPath }],
    thumbnailPaths: [oldPath],
  });
}

export function buildWorkspacePathRewritesDescriptor(
  rewrites: WorkspacePathRewriteLike[],
  events: RuntimeRefreshEvent[],
): RuntimeEffectDescriptor {
  if (rewrites.length === 0) {
    return buildRefreshDescriptor(events);
  }

  return mergeRuntimeEffectDescriptors(buildRefreshDescriptor(events), {
    ...EMPTY_RUNTIME_EFFECT_DESCRIPTOR,
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
  return mergeRuntimeEffectDescriptors(buildRefreshDescriptor(events), {
    ...EMPTY_RUNTIME_EFFECT_DESCRIPTOR,
    invalidatedPaths: [path],
    thumbnailPaths: [path],
  });
}

export function buildObjectCountDeltaDescriptor(
  objectId: string,
  delta: number,
  events: RuntimeRefreshEvent[],
): RuntimeEffectDescriptor {
  return mergeRuntimeEffectDescriptors(buildRefreshDescriptor(events), {
    ...EMPTY_RUNTIME_EFFECT_DESCRIPTOR,
    objectCountDeltas: [{ objectId, delta }],
  });
}

export function buildQueryInvalidationDescriptor(
  queryKeys: Array<readonly unknown[]>,
  events: RuntimeRefreshEvent[],
): RuntimeEffectDescriptor {
  return mergeRuntimeEffectDescriptors(buildRefreshDescriptor(events), {
    ...EMPTY_RUNTIME_EFFECT_DESCRIPTOR,
    invalidatedQueryKeys: queryKeys,
  });
}

export function buildQueryRemovalDescriptor(
  queryKeys: Array<readonly unknown[]>,
  events: RuntimeRefreshEvent[],
): RuntimeEffectDescriptor {
  return mergeRuntimeEffectDescriptors(buildRefreshDescriptor(events), {
    ...EMPTY_RUNTIME_EFFECT_DESCRIPTOR,
    removedQueryKeys: queryKeys,
  });
}
