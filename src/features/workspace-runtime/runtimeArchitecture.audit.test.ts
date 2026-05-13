import { describe, expect, it } from 'vitest';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';

const WORKSPACE_ROOT = process.cwd();
const MODS_RUNTIME_DIRECTORIES = [
  'src/features/object-list',
  'src/features/folder-grid',
  'src/features/preview',
  'src/features/mod-runtime',
  'src/features/file-watcher',
  'src/features/workspace-runtime',
  'src/hooks',
];

const MODS_RUNTIME_CONSUMER_DIRECTORIES = [
  'src/features/object-list',
  'src/features/folder-grid',
  'src/features/preview',
  'src/features/mod-runtime',
  'src/features/workspace-runtime',
];

const FRONTEND_AUDIT_DIRECTORIES = ['src'];
const WORKSPACE_REQUIREMENT_DOCS = [
  '.docs/flow.md',
  '.docs/requirements/req-05-workspace-layout.md',
  '.docs/requirements/req-06-objectlist-navigation.md',
  '.docs/requirements/req-07-object-list.md',
  '.docs/requirements/req-08-smart-filters.md',
  '.docs/requirements/req-11-folder-listing.md',
  '.docs/requirements/req-12-folder-grid-ui.md',
  '.docs/requirements/req-13-core-mod-ops.md',
  '.docs/requirements/req-14-bulk-operations.md',
  '.docs/requirements/req-15-foldergrid-interactions.md',
  '.docs/requirements/req-16-preview-panel-layout.md',
  '.docs/requirements/req-20-mod-toggle.md',
  '.docs/requirements/req-21-mod-rename.md',
  '.docs/requirements/req-28-file-watcher.md',
];
const RUNTIME_CONTRACT_DOCS = [
  ...WORKSPACE_REQUIREMENT_DOCS,
  '.docs/requirements/req-25-scan-engine.md',
  '.docs/requirements/req-27-sync-database.md',
  '.docs/requirements/req-29-conflict-detection.md',
  '.docs/requirements/req-31-collections.md',
  '.docs/requirements/req-33-dashboard.md',
  '.docs/requirements/req-38-auto-organizer.md',
  '.docs/requirements/req-40-metadata-actions.md',
];

const EXCLUDED_FILES = new Set<string>([
  'src/features/runtime-sync/queryRefresh.ts',
  'src/hooks/useFolders.ts',
  'src/hooks/useObjects.ts',
]);

const USER_INPUT_FILESYSTEM_PREFLIGHT_FILES = new Set<string>([
  'src/features/object-list/useObjHandlersArchive.ts',
  'src/features/object-list/useObjHandlersDrop.ts',
]);

function collectSourceFiles(directory: string): string[] {
  const absoluteDirectory = join(WORKSPACE_ROOT, directory);
  const entries = readdirSync(absoluteDirectory);
  const files: string[] = [];

  for (const entry of entries) {
    const absolutePath = join(absoluteDirectory, entry);
    const stats = statSync(absolutePath);
    if (stats.isDirectory()) {
      files.push(...collectSourceFiles(relative(WORKSPACE_ROOT, absolutePath)));
      continue;
    }

    if (!/\.(ts|tsx)$/.test(entry)) {
      continue;
    }

    if (/\.test\.(ts|tsx)$/.test(entry)) {
      continue;
    }

    const relativePath = relative(WORKSPACE_ROOT, absolutePath).replace(/\\/g, '/');
    if (EXCLUDED_FILES.has(relativePath)) {
      continue;
    }

    files.push(absolutePath);
  }

  return files;
}

function readRuntimeSources(): Array<{ path: string; source: string }> {
  const files = MODS_RUNTIME_DIRECTORIES.flatMap((directory) => collectSourceFiles(directory));
  return files.map((absolutePath) => ({
    path: relative(WORKSPACE_ROOT, absolutePath).replace(/\\/g, '/'),
    source: readFileSync(absolutePath, 'utf8'),
  }));
}

function readRuntimeConsumerSources(): Array<{ path: string; source: string }> {
  const files = MODS_RUNTIME_CONSUMER_DIRECTORIES.flatMap((directory) =>
    collectSourceFiles(directory),
  );
  return files.map((absolutePath) => ({
    path: relative(WORKSPACE_ROOT, absolutePath).replace(/\\/g, '/'),
    source: readFileSync(absolutePath, 'utf8'),
  }));
}

function readFrontendAuditSources(): Array<{ path: string; source: string }> {
  const files = FRONTEND_AUDIT_DIRECTORIES.flatMap((directory) => collectSourceFiles(directory));
  return files.map((absolutePath) => ({
    path: relative(WORKSPACE_ROOT, absolutePath).replace(/\\/g, '/'),
    source: readFileSync(absolutePath, 'utf8'),
  }));
}

function readAuditSources(paths: string[]): Array<{ path: string; source: string }> {
  return paths.map((filePath) => ({
    path: filePath,
    source: readFileSync(join(WORKSPACE_ROOT, filePath), 'utf8'),
  }));
}

describe('mods runtime architecture audit', () => {
  it('does not call publishRuntimeEvents directly from consumer code', () => {
    const offenders = readRuntimeSources()
      .filter((file) => file.source.includes('publishRuntimeEvents('))
      .map((file) => file.path);

    expect(offenders).toEqual([]);
  });

  it('does not use refreshObjectListQueries in mods runtime consumer code', () => {
    const offenders = readRuntimeSources()
      .filter((file) => file.source.includes('refreshObjectListQueries('))
      .map((file) => file.path);

    expect(offenders).toEqual([]);
  });

  it('does not use refreshRuntimeQueries directly from consumer code', () => {
    const offenders = readRuntimeSources()
      .filter((file) => file.source.includes('refreshRuntimeQueries('))
      .map((file) => file.path);

    expect(offenders).toEqual([]);
  });

  it('does not reference removed runtimeSelection helpers', () => {
    const offenders = readRuntimeSources()
      .filter(
        (file) =>
          file.source.includes('runtimeSelection') ||
          file.source.includes('focusWorkspaceObject(') ||
          file.source.includes('syncExplorerToObjectRoot(') ||
          file.source.includes('applyWorkspaceExplorerLocation(') ||
          file.source.includes('clearWorkspaceSelection('),
      )
      .map((file) => file.path);

    expect(offenders).toEqual([]);
  });

  it('does not depend on legacy object/folder query helpers in consumer surfaces', () => {
    const offenders = readRuntimeConsumerSources()
      .filter(
        (file) =>
          file.source.includes('useObjects(') ||
          file.source.includes('useModFolders(') ||
          file.source.includes('commands.getObjects('),
      )
      .map((file) => file.path);

    expect(offenders).toEqual([]);
  });

  it('does not resolve workspace mod paths through legacy object mod path IPC in consumers', () => {
    const offenders = readRuntimeConsumerSources()
      .filter((file) => file.source.includes('commands.listObjectModPaths('))
      .map((file) => file.path);

    expect(offenders).toEqual([]);
  });

  it('does not keep legacy object mod path IPC in active source', () => {
    const offenders = readAuditSources([
      'src/lib/bindings.ts',
      'src-tauri/src/lib.rs',
      'src-tauri/src/commands/mods/mod_meta_cmds.rs',
      'src-tauri/permissions/app-commands.toml',
    ])
      .filter(
        (file) =>
          file.source.includes('listObjectModPaths') ||
          file.source.includes('list_object_mod_paths') ||
          file.source.includes('mod_meta_cmds::list_object_mod_paths'),
      )
      .map((file) => file.path);

    expect(offenders).toEqual([]);
  });

  it('does not allow custom bindings to drift back to raw Specta output', () => {
    const bindingsSource = readFileSync(join(WORKSPACE_ROOT, 'src/lib/bindings.ts'), 'utf8');

    expect(bindingsSource.includes('This file was generated by [tauri-specta]')).toBe(false);
    expect(bindingsSource.includes('Promise<Result<')).toBe(false);
    expect(bindingsSource.includes(' as any')).toBe(false);
  });

  it('does not import internal runtime helpers through public useFolders/useObjects barrels', () => {
    const offenders = readRuntimeSources()
      .filter(
        (file) =>
          file.source.includes('hooks/useFolders') || file.source.includes('hooks/useObjects'),
      )
      .map((file) => file.path);

    expect(offenders).toEqual([]);
  });

  it('does not open runtime dialogs through useAppStore directly in mods consumers', () => {
    const offenders = readRuntimeConsumerSources()
      .filter(
        (file) =>
          file.source.includes('openConflictDialog(') ||
          file.source.includes('openDuplicateConflictDialog(') ||
          file.source.includes('openFileInUseDialog('),
      )
      .map((file) => file.path);

    expect(offenders).toEqual([]);
  });

  it('does not keep legacy conflict dialog state in app store', () => {
    const appStoreSource = readFileSync(join(WORKSPACE_ROOT, 'src/stores/useAppStore.ts'), 'utf8');

    expect(appStoreSource.includes('conflictDialog:')).toBe(false);
    expect(appStoreSource.includes('duplicateConflictDialog:')).toBe(false);
    expect(appStoreSource.includes('fileInUseDialog:')).toBe(false);
    expect(appStoreSource.includes('openConflictDialog:')).toBe(false);
    expect(appStoreSource.includes('openDuplicateConflictDialog:')).toBe(false);
    expect(appStoreSource.includes('openFileInUseDialog:')).toBe(false);
  });

  it('does not keep dead runtime rewrite or drag state in app store', () => {
    const appStoreSource = readFileSync(join(WORKSPACE_ROOT, 'src/stores/useAppStore.ts'), 'utf8');

    expect(appStoreSource.includes('workspacePendingSelectionRewrite')).toBe(false);
    expect(appStoreSource.includes('workspaceDragDropState')).toBe(false);
  });

  it('does not reintroduce removed preview wrapper or selection helper', () => {
    const offenders = readRuntimeSources()
      .filter(
        (file) =>
          file.source.includes('usePreviewPanelActions(') ||
          file.source.includes('useSelectedModPath('),
      )
      .map((file) => file.path);

    expect(offenders).toEqual([]);
  });

  it('does not reintroduce removed switch compatibility commands in frontend source', () => {
    const offenders = readFrontendAuditSources()
      .filter(
        (file) =>
          file.source.includes('commands.toggleMod(') ||
          file.source.includes('commands.enableOnlyThis(') ||
          file.source.includes('commands.checkDuplicateEnabled('),
      )
      .map((file) => file.path);

    expect(offenders).toEqual([]);
  });

  it('does not use raw invalidateQueries outside central infra', () => {
    const offenders = readFrontendAuditSources()
      .filter(
        (file) =>
          file.path !== 'src/features/runtime-sync/queryRefresh.ts' &&
          file.path !== 'src/features/workspace-runtime/optimistic/applyOptimisticEffects.ts' &&
          file.source.includes('invalidateQueries('),
      )
      .map((file) => file.path);

    expect(offenders).toEqual([]);
  });

  it('does not run workspace stale selection repair through frontend filesystem probes', () => {
    const offenders = readRuntimeConsumerSources()
      .filter((file) => !USER_INPUT_FILESYSTEM_PREFLIGHT_FILES.has(file.path))
      .filter(
        (file) =>
          file.source.includes('useObjectSelectionRepair(') ||
          file.source.includes('checkPathExists(') ||
          file.source.includes('requestRepairSync'),
      )
      .map((file) => file.path);

    expect(offenders).toEqual([]);
  });

  it('does not expose frontend command wrapper aliases ending in Cmd', () => {
    const bindingsSource = readFileSync(join(WORKSPACE_ROOT, 'src/lib/bindings.ts'), 'utf8');
    const aliasMatches = bindingsSource.match(/\b[a-zA-Z][a-zA-Z0-9]*Cmd\s*:/g) ?? [];

    expect(aliasMatches).toEqual([]);
  });

  it('does not call listModFolders from workspace consumer code', () => {
    const offenders = readRuntimeConsumerSources()
      .filter((file) => file.source.includes('commands.listModFolders('))
      .map((file) => file.path);

    expect(offenders).toEqual([]);
  });

  it('keeps watcher suppression behind the central helper', () => {
    const allowedFiles = new Set<string>(['src/features/file-watcher/watcherSuppression.ts']);
    const offenders = readFrontendAuditSources()
      .filter((file) => !allowedFiles.has(file.path))
      .filter((file) => file.source.includes('commands.setWatcherSuppression('))
      .map((file) => file.path);

    expect(offenders).toEqual([]);
  });

  it('does not reintroduce legacy watcher command aliases or dead bindings', () => {
    const stalePatterns = [
      'startWatcherCmd',
      'stopWatcherCmd',
      'setWatcherSuppressionCmd',
      'undoCollection',
    ];
    const offenders = readFrontendAuditSources()
      .filter((file) => stalePatterns.some((pattern) => file.source.includes(pattern)))
      .map((file) => file.path);

    expect(offenders).toEqual([]);
  });

  it('does not build literal refresh event arrays in feature code', () => {
    const offenders = readFrontendAuditSources()
      .filter(
        (file) =>
          file.path !== 'src/features/workspace-runtime/optimistic/descriptorBuilders.ts' &&
          file.path !== 'src/features/workspace-runtime/optimistic/descriptor.ts' &&
          (file.source.includes('buildRuntimeRefreshDescriptor([') ||
            file.source.includes('refreshEvents: [')),
      )
      .map((file) => file.path);

    expect(offenders).toEqual([]);
  });

  it('keeps active docs aligned with workspace runtime architecture', () => {
    const offenders = readAuditSources([
      ...WORKSPACE_REQUIREMENT_DOCS,
      '.docs/requirements/req-17-metadata-editor.md',
      '.docs/requirements/req-18-ini-viewer.md',
      '.docs/requirements/req-19-image-gallery.md',
      '.docs/requirements/req-22-trash-safety.md',
      '.docs/requirements/req-23-mod-import.md',
      '.docs/requirements/req-39-folder-collision.md',
    ])
      .filter(
        (file) =>
          file.source.includes('useObjects(') ||
          file.source.includes('useFolderGridActions.ts') ||
          file.source.includes('refreshObjectListQueries(') ||
          file.source.includes('selectedFolders') ||
          file.source.includes('toggle_mod') ||
          file.source.includes('invalidateQueries(') ||
          file.source.includes('invalidateQueries(['),
      )
      .map((file) => file.path);

    expect(offenders).toEqual([]);
  });

  it('keeps workspace docs free of removed APIs and layout dependencies', () => {
    const stalePatterns = [
      'react-resizable-panels',
      'getObjectsCmd',
      'commands.listFolders',
      'queryClient.invalidateQueries',
      'toggle_mod',
      'useObjects(',
      'useModFolders(',
    ];
    const offenders = readAuditSources(WORKSPACE_REQUIREMENT_DOCS)
      .filter((file) => stalePatterns.some((pattern) => file.source.includes(pattern)))
      .map((file) => file.path);

    expect(offenders).toEqual([]);
  });

  it('keeps active runtime docs free of removed IPC and raw refresh claims', () => {
    const stalePatterns = [
      /queryClient\.invalidateQueries\(?/,
      /invalidateQueries\(/,
      /`toggle_mod`/,
      /toggle_mod command/,
      /toggle_mod\(/,
      /`undo_collection`/,
      /undo_collection command/,
      /undo_collection\(/,
      /listObjectModPaths/,
      /list_object_mod_paths/,
    ];
    const offenders = readAuditSources(RUNTIME_CONTRACT_DOCS)
      .filter((file) => stalePatterns.some((pattern) => pattern.test(file.source)))
      .map((file) => file.path);

    expect(offenders).toEqual([]);
  });

  it('keeps watcher source trigger-only without scanner entrypoints', () => {
    const offenders = readAuditSources([
      'src/features/file-watcher/hooks.ts',
      'src-tauri/src/services/scanner/watcher/lifecycle.rs',
      'src-tauri/src/commands/scanner/watcher_cmds.rs',
    ])
      .filter(
        (file) =>
          file.source.includes('deepmatch_preview') ||
          file.source.includes('deepmatch_scanner') ||
          file.source.includes('runDeepmatch'),
      )
      .map((file) => file.path);

    expect(offenders).toEqual([]);
  });

  it('keeps menu policy hooks free of imperative runtime side effects', () => {
    const offenders = readAuditSources([
      'src/hooks/useModContextMenuItems.ts',
      'src/features/mod-runtime/actions/modContextMenuPolicy.ts',
    ])
      .filter(
        (file) =>
          file.source.includes('commands.') ||
          file.source.includes('navigator.clipboard') ||
          file.source.includes('openDialog(') ||
          file.source.includes('alert(') ||
          file.source.includes('console.error(') ||
          file.source.includes('publishRuntimeDescriptor('),
      )
      .map((file) => file.path);

    expect(offenders).toEqual([]);
  });

  it('keeps surface components free of direct preview import/event wiring', () => {
    const offenders = readAuditSources([
      'src/features/preview/PreviewPanel.tsx',
      'src/features/folder-grid/FolderGrid.tsx',
      'src/features/object-list/ObjectList.tsx',
    ])
      .filter(
        (file) =>
          file.source.includes('window.dispatchEvent(') ||
          file.source.includes('publishRuntimeDescriptor(') ||
          file.source.includes('window.addEventListener('),
      )
      .map((file) => file.path);

    expect(offenders).toEqual([]);
  });

  it('keeps folder-grid dialogs free of direct runtime descriptor publishing and console logging', () => {
    const offenders = readAuditSources([
      'src/features/folder-grid/ConflictResolveDialog.tsx',
      'src/features/folder-grid/ObjectConflictModal.tsx',
      'src/features/folder-grid/MoveToObjectDialog.tsx',
      'src/features/folder-grid/IgnoreManagementModal.tsx',
    ])
      .filter(
        (file) =>
          file.source.includes('publishRuntimeDescriptor(') ||
          file.source.includes('console.error('),
      )
      .map((file) => file.path);

    expect(offenders).toEqual([]);
  });
});
