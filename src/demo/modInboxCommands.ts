import type {
  ModInboxSnapshot,
  ImportBatch,
  CreateModInboxBatchInput,
  ImportItem,
  ImportBatchReport,
} from '@/shared/api/tauri/bindings.gen';
export let demoModInbox: ModInboxSnapshot = {
  gameId: 'demo-zenless',
  rootPath: 'C:\\Demo\\Zenless\\ReadyToMove',
  rootState: 'ready',
  readyEntries: [
    {
      entryKey: 'demo-inbox-archive',
      name: 'Nekomata Streetwear.zip',
      path: 'C:\\Demo\\Zenless\\ReadyToMove\\Nekomata Streetwear.zip',
      kind: 'archive',
      archiveFormat: 'zip',
      sizeBytes: 48_234_496,
      modifiedUnixMs: '1789282800000',
      layout: 'wrapper',
      detectedRootCount: 1,
      pendingBatchId: null,
    },
    {
      entryKey: 'demo-inbox-folder',
      name: 'Lumina Square Recolor',
      path: 'C:\\Demo\\Zenless\\ReadyToMove\\Lumina Square Recolor',
      kind: 'folder',
      archiveFormat: null,
      sizeBytes: 12_582_912,
      modifiedUnixMs: '1789279200000',
      layout: 'direct_mod',
      detectedRootCount: 1,
      pendingBatchId: null,
    },
  ],
  processedSources: [],
};

export let demoModInboxBatch: ImportBatch | null = null;

export function readCreateModInboxBatchInput(input: unknown): CreateModInboxBatchInput | null {
  if (typeof input !== 'object' || input === null) return null;
  const candidate = input as Record<string, unknown>;
  if (
    typeof candidate.gameId !== 'string' ||
    !Array.isArray(candidate.entryKeys) ||
    !candidate.entryKeys.every((entryKey) => typeof entryKey === 'string')
  ) {
    return null;
  }
  return { gameId: candidate.gameId, entryKeys: candidate.entryKeys };
}

function demoDestination(entryKey: string): {
  objectId: string;
  objectName: string;
  folderName: string;
  targetPath: string;
} {
  return entryKey === 'demo-inbox-archive'
    ? {
        objectId: 'demo-object-nekomata',
        objectName: 'Nekomata',
        folderName: 'Nekomata',
        targetPath: 'Characters/Nekomata/Streetwear',
      }
    : {
        objectId: 'demo-object-lumina',
        objectName: 'Lumina Square',
        folderName: 'Lumina Square',
        targetPath: 'Environment/Lumina Square/Recolor',
      };
}

function createDemoModInboxItem(
  entry: ModInboxSnapshot['readyEntries'][number],
  batchId: string,
): ImportItem {
  const destination = demoDestination(entry.entryKey);
  return {
    id: `demo-import-${entry.entryKey}`,
    batchId,
    sourceKind: entry.kind === 'archive' ? 'archive_root' : 'folder',
    sourcePath: entry.path,
    stagingPath: null,
    plannedName: entry.name,
    status: 'ready',
    matchCategory: 'Other',
    subCategory: null,
    classificationMetadata: {},
    sourceMetadata: {},
    categorySuggestions: [],
    canonicalSuggestions: [],
    destinationSuggestions: [
      {
        kind: 'existing_object',
        objectId: destination.objectId,
        canonicalEntryKey: null,
        folderName: destination.folderName,
        targetPath: destination.targetPath,
        confidencePercentage: 94,
        confidenceTier: 'high',
        warning: null,
      },
    ],
    selectedEntryKey: null,
    selectedAliasName: null,
    destinationObjectId: destination.objectId,
    destinationPath: destination.targetPath,
    confidencePercentage: 94,
    confidenceTier: 'high',
    identityMatchStatus: 'auto_matched',
    evidence: [],
    decision: 'reallocate',
    fingerprint: null,
    archiveSha256: null,
    payloadManifest: null,
    duplicateOfItemId: null,
    targetComparison: null,
    analysisRevision: 1,
    analysisAckRevision: 1,
    reviewGate: { reasons: [] },
    diagnostics: [],
    contentKind: 'unknown',
    packageShape: 'single',
    result: null,
    error: null,
  };
}

export function createDemoModInboxBatch(input: CreateModInboxBatchInput): ImportBatch {
  if (input.gameId !== demoModInbox.gameId) {
    throw new Error('The selected demo game does not own this Mod Inbox.');
  }
  const entryKeys = new Set(input.entryKeys);
  const selectedEntries = demoModInbox.readyEntries.filter(
    (entry) => entryKeys.has(entry.entryKey) && entry.pendingBatchId === null,
  );
  if (selectedEntries.length === 0 || selectedEntries.length !== entryKeys.size) {
    throw new Error('Select one or more available Mod Inbox entries.');
  }

  const id = 'demo-mod-inbox-batch';
  const timestamp = '2026-10-07T00:00:00Z';
  demoModInboxBatch = {
    id,
    gameId: input.gameId,
    flow: 'ready_to_move',
    targetMode: 'auto',
    targetObjectId: null,
    targetSubpath: null,
    status: 'awaiting_review',
    sourceArchivePath: null,
    items: selectedEntries.map((entry) => createDemoModInboxItem(entry, id)),
    createdAt: timestamp,
    updatedAt: timestamp,
  };
  demoModInbox = {
    ...demoModInbox,
    readyEntries: demoModInbox.readyEntries.map((entry) =>
      entryKeys.has(entry.entryKey) ? { ...entry, pendingBatchId: id } : entry,
    ),
  };
  return demoModInboxBatch;
}

export function readBatchId(input: unknown): string | null {
  if (typeof input === 'string') return input;
  if (typeof input !== 'object' || input === null) return null;
  const candidate = input as Record<string, unknown>;
  return typeof candidate.batchId === 'string' ? candidate.batchId : null;
}

export function completeDemoModInboxBatch(batchId: string): ImportBatchReport {
  if (!demoModInboxBatch || demoModInboxBatch.id !== batchId) {
    throw new Error('The requested demo import batch does not exist.');
  }
  const completedItems = demoModInboxBatch.items.map((item) => ({
    ...item,
    status: 'done' as const,
    result: 'Moved to the selected demo destination.',
  }));
  demoModInboxBatch = {
    ...demoModInboxBatch,
    status: 'done',
    items: completedItems,
    updatedAt: '2026-10-07T00:01:00Z',
  };
  const completedIds = new Set(completedItems.map((item) => item.id));
  demoModInbox = {
    ...demoModInbox,
    readyEntries: demoModInbox.readyEntries.filter(
      (entry) => !completedIds.has(`demo-import-${entry.entryKey}`),
    ),
    processedSources: [
      ...demoModInbox.processedSources,
      ...completedItems.map((item) => {
        const destination = demoDestination(item.id.replace('demo-import-', ''));
        return {
          sourceId: item.id,
          name: item.plannedName,
          sourceKind:
            item.sourceKind === 'archive_root' ? ('archive' as const) : ('folder' as const),
          originalPath: item.sourcePath,
          processedPath: `${demoModInbox.rootPath}\\Processed\\${item.plannedName}`,
          processedAt: '2026-10-07T00:01:00Z',
          sourceDeletedAt: null,
          destinations: [
            {
              objectId: destination.objectId,
              objectName: destination.objectName,
              placedPath: destination.targetPath,
              plannedName: item.plannedName,
              status: 'done' as const,
            },
          ],
        };
      }),
    ],
  };
  return {
    batchId,
    moved: completedItems.length,
    reallocated: completedItems.length,
    createdCanonicalFolders: 0,
    skipped: 0,
    collisions: 0,
    metadataPending: 0,
    failed: 0,
  };
}
