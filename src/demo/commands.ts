import type {
  CollectionPreview,
  CollectionSummary,
  BrowserDownloadDto,
  CollectionRuntimeDescriptor,
  CollectionRuntimeSnapshot,
  CreateModInboxBatchInput,
  DupScanReport,
  ImportBatch,
  ImportBatchReport,
  ImportItem,
  ModHealthReport,
  ModInboxSnapshot,
  ProjectedCollectionState,
  SaveSettingsResult,
} from '@/shared/api/tauri/bindings.gen';
import { demoDashboardGateway } from './dashboard';
import { demoGameSettings, getDemoSettings, setDemoSettings } from './game';
import {
  buildDemoWorkspacePreview,
  buildDemoWorkspaceStructure,
  demoGameSchema,
} from './workspace';

export interface DemoCommandResult {
  handled: boolean;
  value: unknown;
}

function handled(value: unknown): DemoCommandResult {
  return { handled: true, value };
}

const demoModHealth: ModHealthReport = {
  support_level: 'supported',
  issues: [],
  manifest: {
    referenced: [],
    inactive_only: [],
    orphan: [],
    external_reference: [],
    counts: {
      referenced: 0,
      inactive_only: 0,
      orphan: 0,
      external_reference: 0,
    },
  },
  file_manifest: [],
  controls: [],
};

const demoCollections: CollectionSummary[] = [
  {
    id: 'demo-collection-story',
    name: 'Story Mode',
    is_safe: true,
    is_safety_classified: true,
    is_active: true,
    signature: 'demo-story-v1',
    updated_at: '2026-09-13T06:00:00Z',
    mod_count: 84,
  },
  {
    id: 'demo-collection-photo',
    name: 'Photo Session',
    is_safe: true,
    is_safety_classified: true,
    is_active: false,
    signature: 'demo-photo-v1',
    updated_at: '2026-09-12T06:00:00Z',
    mod_count: 31,
  },
  {
    id: 'demo-collection-exploration',
    name: 'Exploration Set',
    is_safe: true,
    is_safety_classified: true,
    is_active: false,
    signature: 'demo-exploration-v1',
    updated_at: '2026-09-11T08:00:00Z',
    mod_count: 18,
  },
];

const demoProjectedState: ProjectedCollectionState = {
  object_states: [
    {
      object_id: 'demo-object-nekomata',
      display_name: 'Nekomata',
      path_key: 'characters/nekomata',
      is_enabled: true,
      active_root_count: 1,
    },
    {
      object_id: 'demo-object-lumina',
      display_name: 'Lumina Square',
      path_key: 'environment/lumina-square',
      is_enabled: true,
      active_root_count: 2,
    },
    {
      object_id: 'demo-object-interface',
      display_name: 'Interface',
      path_key: 'ui/interface',
      is_enabled: true,
      active_root_count: 1,
    },
  ],
  active_roots: [
    {
      object_id: 'demo-object-nekomata',
      root_key: 'characters/nekomata/streetwear',
      display_name: 'Streetwear',
      root_type: 'mod',
      source_path: 'Characters\\Nekomata\\Streetwear',
      thumbnail_hint: null,
      warnings: [],
      is_missing: false,
      is_safe: true,
      safety_source: 'demo',
    },
    {
      object_id: 'demo-object-lumina',
      root_key: 'environment/lumina-square/recolor',
      display_name: 'Lumina Square Recolor',
      root_type: 'mod',
      source_path: 'Environment\\Lumina Square\\Recolor',
      thumbnail_hint: null,
      warnings: [],
      is_missing: false,
      is_safe: true,
      safety_source: 'demo',
    },
    {
      object_id: 'demo-object-lumina',
      root_key: 'environment/lumina-square/rainy-evening',
      display_name: 'Rainy Evening',
      root_type: 'mod',
      source_path: 'Environment\\Lumina Square\\Rainy Evening',
      thumbnail_hint: null,
      warnings: [],
      is_missing: false,
      is_safe: true,
      safety_source: 'demo',
    },
    {
      object_id: 'demo-object-interface',
      root_key: 'ui/interface/minimal-hud',
      display_name: 'Minimal HUD',
      root_type: 'mod',
      source_path: 'UI\\Interface\\Minimal HUD',
      thumbnail_hint: null,
      warnings: [],
      is_missing: false,
      is_safe: true,
      safety_source: 'demo',
    },
  ],
  summary: {
    object_count: 3,
    enabled_object_count: 3,
    active_root_count: 4,
    missing_root_count: 0,
  },
};

const demoCollectionRuntimeDescriptor: CollectionRuntimeDescriptor = {
  game_id: 'demo-zenless',
  active_collection_id: 'demo-collection-story',
  active_collection_name: 'Story Mode',
  runtime_status: 'clean',
  missing_count: 0,
  safety: { is_safe: true, is_safety_classified: true },
  counts: { active_mod_count: 4, object_count: 3, enabled_object_count: 3 },
  last_changes: null,
};

function getDemoCollectionPreview(collectionId: string): CollectionPreview {
  const collection = demoCollections.find((item) => item.id === collectionId) ?? demoCollections[0];
  return {
    collection,
    tree_nodes: [
      {
        kind: 'object',
        id: 'demo-object-nekomata',
        name: 'Nekomata',
        path: 'Characters\\Nekomata',
        object_id: 'demo-object-nekomata',
        node_type: 'character',
        is_enabled: true,
        is_effectively_active: true,
        inactive_reason: null,
        show_inactive_chip: false,
        status_kind: null,
        collapse_children: false,
        warnings: [],
        mod_count: 2,
        children: [
          {
            kind: 'mod',
            id: 'demo-mod-1',
            name: 'Nekomata Streetwear',
            path: 'Characters\\Nekomata\\Streetwear',
            object_id: 'demo-object-nekomata',
            node_type: 'mod',
            is_enabled: true,
            is_effectively_active: true,
            inactive_reason: null,
            show_inactive_chip: false,
            status_kind: null,
            collapse_children: false,
            warnings: [],
            mod_count: null,
            children: [],
          },
          {
            kind: 'mod',
            id: 'demo-mod-summer',
            name: 'Summer Palette',
            path: 'Characters\\Nekomata\\Summer Palette',
            object_id: 'demo-object-nekomata',
            node_type: 'mod',
            is_enabled: false,
            is_effectively_active: false,
            inactive_reason: 'Disabled',
            show_inactive_chip: true,
            status_kind: null,
            collapse_children: false,
            warnings: [],
            mod_count: null,
            children: [],
          },
        ],
      },
      {
        kind: 'object',
        id: 'demo-object-lumina',
        name: 'Lumina Square',
        path: 'Environment\\Lumina Square',
        object_id: 'demo-object-lumina',
        node_type: 'environment',
        is_enabled: true,
        is_effectively_active: true,
        inactive_reason: null,
        show_inactive_chip: false,
        status_kind: null,
        collapse_children: false,
        warnings: [],
        mod_count: 2,
        children: [
          {
            kind: 'mod',
            id: 'demo-mod-lumina-recolor',
            name: 'Lumina Square Recolor',
            path: 'Environment\\Lumina Square\\Recolor',
            object_id: 'demo-object-lumina',
            node_type: 'mod',
            is_enabled: true,
            is_effectively_active: true,
            inactive_reason: null,
            show_inactive_chip: false,
            status_kind: null,
            collapse_children: false,
            warnings: [],
            mod_count: null,
            children: [],
          },
          {
            kind: 'mod',
            id: 'demo-mod-rainy-evening',
            name: 'Rainy Evening',
            path: 'Environment\\Lumina Square\\Rainy Evening',
            object_id: 'demo-object-lumina',
            node_type: 'mod',
            is_enabled: true,
            is_effectively_active: true,
            inactive_reason: null,
            show_inactive_chip: false,
            status_kind: null,
            collapse_children: false,
            warnings: [],
            mod_count: null,
            children: [],
          },
        ],
      },
      {
        kind: 'object',
        id: 'demo-object-interface',
        name: 'Interface',
        path: 'UI\\Interface',
        object_id: 'demo-object-interface',
        node_type: 'ui',
        is_enabled: true,
        is_effectively_active: true,
        inactive_reason: null,
        show_inactive_chip: false,
        status_kind: null,
        collapse_children: false,
        warnings: [],
        mod_count: 1,
        children: [
          {
            kind: 'mod',
            id: 'demo-mod-minimal-hud',
            name: 'Minimal HUD',
            path: 'UI\\Interface\\Minimal HUD',
            object_id: 'demo-object-interface',
            node_type: 'mod',
            is_enabled: true,
            is_effectively_active: true,
            inactive_reason: null,
            show_inactive_chip: false,
            status_kind: null,
            collapse_children: false,
            warnings: [],
            mod_count: null,
            children: [],
          },
        ],
      },
    ],
    projected_state: demoProjectedState,
  };
}

function getDemoCollectionRuntimeState(): CollectionRuntimeSnapshot {
  const preview = getDemoCollectionPreview('demo-collection-story');
  return {
    game_id: 'demo-zenless',
    active_collection_id: preview.collection.id,
    active_collection_name: preview.collection.name,
    current_signature: 'demo-story-v1',
    is_dirty: false,
    runtime_status: 'clean',
    is_safe: true,
    is_safety_classified: true,
    missing_count: 0,
    last_changes: null,
    current_mods: [],
    current_objects: [],
    current_tree_nodes: preview.tree_nodes,
    projected_state: preview.projected_state,
  };
}

let demoModInbox: ModInboxSnapshot = {
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

let demoModInboxBatch: ImportBatch | null = null;

function readCreateModInboxBatchInput(input: unknown): CreateModInboxBatchInput | null {
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

function createDemoModInboxBatch(input: CreateModInboxBatchInput): ImportBatch {
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

function readBatchId(input: unknown): string | null {
  if (typeof input === 'string') return input;
  if (typeof input !== 'object' || input === null) return null;
  const candidate = input as Record<string, unknown>;
  return typeof candidate.batchId === 'string' ? candidate.batchId : null;
}

function completeDemoModInboxBatch(batchId: string): ImportBatchReport {
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

const demoDupScanReport: DupScanReport = {
  scanId: 'demo-duplicate-scan',
  gameId: 'demo-zenless',
  rootPath: 'C:\\Demo\\Zenless\\Mods',
  totalGroups: 4,
  totalMembers: 9,
  groups: [
    {
      groupId: 'demo-duplicate-group',
      confidenceScore: 94,
      matchReason: 'Matching file hashes',
      isUnsafe: false,
      signals: [{ key: 'hash', detail: '18 matching files', score: 94 }],
      members: [
        {
          modId: 'demo-mod-1',
          version: 1,
          folderPath: 'Characters\\Nekomata\\Streetwear',
          displayName: 'Nekomata Streetwear',
          totalSizeBytes: 367_001_600,
          fileCount: 18,
          isSafe: true,
          confidenceScore: 94,
          signals: [{ key: 'hash', detail: '18 matching files', score: 94 }],
        },
        {
          modId: 'demo-mod-2',
          version: 1,
          folderPath: 'Archive\\Nekomata Streetwear Copy',
          displayName: 'Nekomata Streetwear Copy',
          totalSizeBytes: 367_001_600,
          fileCount: 18,
          isSafe: true,
          confidenceScore: 94,
          signals: [{ key: 'hash', detail: '18 matching files', score: 94 }],
        },
      ],
    },
    {
      groupId: 'demo-duplicate-lumina',
      confidenceScore: 100,
      matchReason: 'Identical BLAKE3 file set',
      isUnsafe: false,
      signals: [{ key: 'blake3', detail: '24 identical files', score: 100 }],
      members: [
        {
          modId: 'demo-mod-lumina-recolor',
          version: 1,
          folderPath: 'Environment\\Lumina Square\\Recolor',
          displayName: 'Lumina Square Recolor',
          totalSizeBytes: 184_549_376,
          fileCount: 24,
          isSafe: true,
          confidenceScore: 100,
          signals: [{ key: 'blake3', detail: '24 identical files', score: 100 }],
        },
        {
          modId: null,
          version: 1,
          folderPath: 'Downloads\\Lumina Square Recolor',
          displayName: 'Lumina Square Recolor (download)',
          totalSizeBytes: 184_549_376,
          fileCount: 24,
          isSafe: true,
          confidenceScore: 100,
          signals: [{ key: 'blake3', detail: '24 identical files', score: 100 }],
        },
      ],
    },
    {
      groupId: 'demo-duplicate-hud',
      confidenceScore: 78,
      matchReason: 'Matching file names and texture dimensions',
      isUnsafe: false,
      signals: [
        { key: 'filename', detail: '8 matching files', score: 80 },
        { key: 'dimensions', detail: '6 matching textures', score: 76 },
      ],
      members: [
        {
          modId: 'demo-mod-minimal-hud',
          version: 1,
          folderPath: 'UI\\Interface\\Minimal HUD',
          displayName: 'Minimal HUD',
          totalSizeBytes: 38_797_312,
          fileCount: 8,
          isSafe: true,
          confidenceScore: 78,
          signals: [{ key: 'filename', detail: '8 matching files', score: 80 }],
        },
        {
          modId: null,
          version: 1,
          folderPath: 'UI\\Interface\\Compact HUD',
          displayName: 'Compact HUD',
          totalSizeBytes: 36_700_160,
          fileCount: 8,
          isSafe: true,
          confidenceScore: 78,
          signals: [{ key: 'dimensions', detail: '6 matching textures', score: 76 }],
        },
      ],
    },
    {
      groupId: 'demo-duplicate-private',
      confidenceScore: 62,
      matchReason: 'Similar metadata and file layout',
      isUnsafe: true,
      signals: [{ key: 'layout', detail: '12 matching paths', score: 62 }],
      members: [
        {
          modId: 'demo-mod-private-variant',
          version: 1,
          folderPath: 'Characters\\Nekomata\\Private Variant',
          displayName: 'Private Variant',
          totalSizeBytes: 96_468_992,
          fileCount: 12,
          isSafe: false,
          confidenceScore: 62,
          signals: [{ key: 'layout', detail: '12 matching paths', score: 62 }],
        },
        {
          modId: 'demo-mod-private-alternate',
          version: 1,
          folderPath: 'Characters\\Nekomata\\Private Alternate',
          displayName: 'Private Alternate',
          totalSizeBytes: 71_303_168,
          fileCount: 12,
          isSafe: false,
          confidenceScore: 62,
          signals: [{ key: 'layout', detail: '12 matching paths', score: 62 }],
        },
        {
          modId: null,
          version: 1,
          folderPath: 'Archive\\Nekomata Private Variant',
          displayName: 'Private Variant (archive)',
          totalSizeBytes: 96_468_992,
          fileCount: 12,
          isSafe: false,
          confidenceScore: 62,
          signals: [{ key: 'layout', detail: '12 matching paths', score: 62 }],
        },
      ],
    },
  ],
};

let demoDownloads: BrowserDownloadDto[] = [
  {
    id: 'demo-download-active',
    game_id: 'demo-game',
    session_id: null,
    filename: 'Nekomata Streetwear.zip',
    file_path: null,
    source_url: 'https://example.invalid/nekomata-streetwear.zip',
    status: 'in_progress',
    bytes_total: 48_234_496,
    bytes_received: 31_678_464,
    error_msg: null,
    can_resume: false,
    tab_label: null,
    queue_order: 0,
    started_at: '2026-09-13T06:00:00Z',
    finished_at: null,
  },
  {
    id: 'demo-download-finished',
    game_id: 'demo-game',
    session_id: null,
    filename: 'Lumina Square Recolor.zip',
    file_path: 'C:\\Demo\\Downloads\\Lumina Square Recolor.zip',
    source_url: 'https://example.invalid/lumina-square-recolor.zip',
    status: 'finished',
    bytes_total: 12_582_912,
    bytes_received: 12_582_912,
    error_msg: null,
    can_resume: false,
    tab_label: null,
    queue_order: 1,
    started_at: '2026-09-13T05:30:00Z',
    finished_at: '2026-09-13T05:31:00Z',
  },
];

let demoBrowserHomepage = 'https://gamebanana.com';
let demoBrowserRetentionDays = 30;

function readDemoSettings(value: unknown) {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    return null;
  }

  const candidate = value as Record<string, unknown>;
  if (
    typeof candidate.theme !== 'string' ||
    typeof candidate.language !== 'string' ||
    !Array.isArray(candidate.games) ||
    typeof candidate.auto_close_launcher !== 'boolean' ||
    typeof candidate.ai !== 'object' ||
    candidate.ai === null ||
    typeof candidate.safety !== 'object' ||
    candidate.safety === null
  ) {
    return null;
  }

  return candidate as typeof demoGameSettings;
}

function saveDemoSettings(settings: typeof demoGameSettings): SaveSettingsResult {
  return { settings: setDemoSettings(settings), sync_warning: null };
}

/**
 * Development-only command data. This is an adapter behind the existing
 * bindings contract, never a replacement component or a browser Tauri shim.
 * Each case must be safe to repeat and must keep all state in memory.
 */
export function resolveDemoCommand(name: string, _args: unknown[]): DemoCommandResult {
  switch (name) {
    case 'getSettings':
      return handled(getDemoSettings());
    case 'saveSettings': {
      const settings = readDemoSettings(_args[0]);
      return settings
        ? handled(saveDemoSettings(settings))
        : handled(Promise.reject(new Error('Demo settings payload is invalid.')));
    }
    case 'setAiApiKey':
      return handled(
        setDemoSettings({
          ...getDemoSettings(),
          ai: { ...getDemoSettings().ai, has_api_key: true },
        }),
      );
    case 'deleteAiApiKey':
      return handled(
        setDemoSettings({
          ...getDemoSettings(),
          ai: { ...getDemoSettings().ai, has_api_key: false },
        }),
      );
    case 'testAiConnection':
      return handled(undefined);
    case 'getDashboardStats':
      return handled(demoDashboardGateway.getDashboardStats());
    case 'getActiveKeybindings':
      return handled(demoDashboardGateway.getActiveKeybindings(String(_args[0] ?? '')));
    case 'getGameSchema':
      return handled(demoGameSchema);
    case 'getWorkspaceStructure':
      return handled(buildDemoWorkspaceStructure(_args[0]));
    case 'getWorkspacePreview':
      return handled(buildDemoWorkspacePreview(_args[0]));
    case 'getObjectsCmd':
      return handled({ objects: buildDemoWorkspaceStructure({}).objects, lost_objects: [] });
    case 'getGames':
      return handled(
        getDemoSettings().games.map((game) => ({
          ...game,
          ready_to_move_path: null,
        })),
      );
    case 'listModIniFiles':
    case 'listModPreviewImages':
      return handled([]);
    case 'analyzeModHealth':
      return handled(demoModHealth);
    case 'listCollections':
      return handled(demoCollections);
    case 'getCollectionRuntimeState':
      return handled(getDemoCollectionRuntimeState());
    case 'getCollectionRuntimeDescriptor':
      return handled(demoCollectionRuntimeDescriptor);
    case 'getCollectionPreview':
      return handled(getDemoCollectionPreview(String(_args[0] ?? '')));
    case 'getModInbox':
      return handled(demoModInbox);
    case 'createModInboxBatch': {
      const input = readCreateModInboxBatchInput(_args[0]);
      return input
        ? handled(createDemoModInboxBatch(input))
        : handled(Promise.reject(new Error('Demo Mod Inbox payload is invalid.')));
    }
    case 'getImportBatch': {
      const batchId = readBatchId(_args[0]);
      return demoModInboxBatch && batchId === demoModInboxBatch.id
        ? handled(demoModInboxBatch)
        : handled(Promise.reject(new Error('Demo import batch was not found.')));
    }
    case 'markImportBatchReviewStarted':
      return handled(undefined);
    case 'retryObjectIdentitySuggestions':
      return handled(undefined);
    case 'previewImportLibraryReadiness': {
      const batchId = typeof _args[0] === 'string' ? _args[0] : '';
      return handled({ batchId, items: [], highCount: 0, mediumCount: 0, reviewStarted: true });
    }
    case 'listImportBatches':
      return handled(demoModInboxBatch ? [demoModInboxBatch] : []);
    case 'commitImportBatch': {
      const batchId = readBatchId(_args[0]);
      return batchId
        ? handled(completeDemoModInboxBatch(batchId))
        : handled(Promise.reject(new Error('Demo import batch payload is invalid.')));
    }
    case 'getIgnoredPairs':
      return handled([]);
    case 'dupScanGetReport':
      return handled(demoDupScanReport);
    case 'browserGetAdblockEnabled':
      return handled(true);
    case 'browserListBookmarks':
    case 'browserListHistory':
    case 'browserGetSessionTabs':
      return handled([]);
    case 'browserGetPrivacySummary':
      return handled({
        bookmarks: 0,
        history_entries: 0,
        saved_permissions: 0,
      });
    case 'browserListDownloads':
      return handled(demoDownloads);
    case 'browserGetHomepage':
      return handled(demoBrowserHomepage);
    case 'browserGetRetentionDays':
      return handled(demoBrowserRetentionDays);
    case 'browserSaveSessionTabs':
    case 'browserSetAdblockEnabled':
    case 'browserClearHistory':
    case 'launchGame':
      return handled(undefined);
    case 'setActiveGame': {
      const gameId = typeof _args[0] === 'string' ? _args[0] : null;
      const settings = getDemoSettings();
      if (gameId !== null && !settings.games.some((game) => game.id === gameId)) {
        return handled(Promise.reject(new Error(`Unknown demo game: ${gameId}`)));
      }
      return handled(setDemoSettings({ ...settings, active_game_id: gameId }));
    }
    case 'browserSetHomepage':
      demoBrowserHomepage = String(_args[0] ?? demoBrowserHomepage);
      return handled(undefined);
    case 'browserSetRetentionDays':
      demoBrowserRetentionDays = Number(_args[0]) || demoBrowserRetentionDays;
      return handled(undefined);
    case 'browserClearOldDownloads': {
      const removable = demoDownloads.filter((download) => download.status === 'finished').length;
      demoDownloads = demoDownloads.filter((download) => download.status !== 'finished');
      return handled(removable);
    }
    case 'browserCancelDownload': {
      const id = String(_args[0] ?? '');
      demoDownloads = demoDownloads.map((download) =>
        download.id === id
          ? { ...download, status: 'canceled', finished_at: '2026-09-13T06:15:00Z' }
          : download,
      );
      return handled(undefined);
    }
    case 'browserRetryDownload': {
      const id = String(_args[0] ?? '');
      demoDownloads = demoDownloads.map((download) =>
        download.id === id
          ? { ...download, status: 'in_progress', error_msg: null, finished_at: null }
          : download,
      );
      return handled(undefined);
    }
    case 'browserDeleteDownload': {
      const id = String(_args[0] ?? '');
      demoDownloads = demoDownloads.filter((download) => download.id !== id);
      return handled(undefined);
    }
    default:
      // Fail closed: an unmodelled command must never fall through to Tauri
      // while browser QA is running against fixture data.
      return handled(Promise.reject(new Error(`Demo data has no handler for ${name}.`)));
  }
}
