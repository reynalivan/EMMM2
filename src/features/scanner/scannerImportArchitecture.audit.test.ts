import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';
import { describe, expect, it } from 'vitest';

const WORKSPACE_ROOT = process.cwd();
const DEEP_MATCH_PATTERNS = [
  'deepmatch_preview',
  'deepmatch_scanner',
  'deep_matcher',
  'runDeepmatch',
  'commit_scan_cmd',
  'commitScan(',
];

const PASSIVE_FRONTEND_PATHS = [
  'src/App.tsx',
  'src/components/layout',
  'src/features/file-watcher',
  'src/features/onboarding',
  'src/features/workspace-runtime',
];

const PASSIVE_RUST_PATHS = [
  'src-tauri/src/services/scanner/watcher',
  'src-tauri/src/services/disk_reconcile',
];

const EXPLICIT_FRONTEND_DEEP_MATCH_CALLERS = new Set<string>([
  'src/lib/services/scanService.ts',
  'src/features/scanner/ScannerFeature.tsx',
  'src/features/object-list/useObjHandlersArchive.ts',
  'src/features/object-list/useObjHandlersBulk.ts',
  'src/features/object-list/useObjHandlersDrop.ts',
  'src/features/object-list/useObjHandlersScan.ts',
  'src/features/settings/tabs/GamesTab.tsx',
]);

const SCANNER_IMPORT_REQUIREMENT_DOCS = [
  '.docs/requirements/req-23-mod-import.md',
  '.docs/requirements/req-25-scan-engine.md',
  '.docs/requirements/req-26-deep-matcher.md',
  '.docs/requirements/req-27-sync-database.md',
  '.docs/requirements/req-37-archive-extraction.md',
  '.docs/requirements/req-38-auto-organizer.md',
  '.docs/requirements/req-44-discover-hub-smart-import.md',
];

function readWorkspaceFile(path: string): string {
  return readFileSync(join(WORKSPACE_ROOT, path), 'utf8');
}

function collectFiles(path: string, extensions: RegExp): string[] {
  const absolutePath = join(WORKSPACE_ROOT, path);
  const stats = statSync(absolutePath);

  if (stats.isFile()) {
    return extensions.test(path) ? [absolutePath] : [];
  }

  const files: string[] = [];
  for (const entry of readdirSync(absolutePath)) {
    const child = join(absolutePath, entry);
    const relativeChild = relative(WORKSPACE_ROOT, child).replace(/\\/g, '/');
    const childStats = statSync(child);

    if (childStats.isDirectory()) {
      files.push(...collectFiles(relativeChild, extensions));
      continue;
    }

    if (extensions.test(entry)) {
      files.push(child);
    }
  }

  return files;
}

function collectSourceFiles(
  paths: string[],
  extensions: RegExp,
): Array<{ path: string; source: string }> {
  return paths
    .flatMap((path) => collectFiles(path, extensions))
    .filter((file) => !/[\\/]tests?[\\/]/.test(file))
    .filter((file) => !/\.test\.(ts|tsx)$/.test(file))
    .map((absolutePath) => ({
      path: relative(WORKSPACE_ROOT, absolutePath).replace(/\\/g, '/'),
      source: readFileSync(absolutePath, 'utf8'),
    }));
}

function hasDeepMatchReference(source: string): boolean {
  return DEEP_MATCH_PATTERNS.some((pattern) => source.includes(pattern));
}

describe('scanner/import architecture audit', () => {
  it('keeps passive frontend runtime paths out of Deep Match scanner flows', () => {
    const offenders = collectSourceFiles(PASSIVE_FRONTEND_PATHS, /\.(ts|tsx)$/)
      .filter((file) => hasDeepMatchReference(file.source))
      .map((file) => file.path);

    expect(offenders).toEqual([]);
  });

  it('keeps watcher and Disk Reconcile backend paths trigger-only', () => {
    const offenders = collectSourceFiles(PASSIVE_RUST_PATHS, /\.rs$/)
      .filter((file) => hasDeepMatchReference(file.source))
      .map((file) => file.path);

    expect(offenders).toEqual([]);
  });

  it('keeps app bootstrap Disk Reconcile-only', () => {
    const source = readWorkspaceFile('src-tauri/src/lib.rs');
    const bootStart = source.indexOf('DiskReconcileReason::StartupBoot');
    const bootEnd = source.indexOf('Ok(())', bootStart);
    const bootBlock = source.slice(bootStart, bootEnd);

    expect(hasDeepMatchReference(bootBlock)).toBe(false);
  });

  it('keeps frontend Deep Match calls in explicit scan/import surfaces', () => {
    const offenders = collectSourceFiles(['src'], /\.(ts|tsx)$/)
      .filter(
        (file) =>
          file.source.includes('scanService.runDeepmatch') ||
          file.source.includes('scanService.commitScan('),
      )
      .filter((file) => !EXPLICIT_FRONTEND_DEEP_MATCH_CALLERS.has(file.path))
      .map((file) => file.path);

    expect(offenders).toEqual([]);
  });

  it('keeps scanner/import docs explicit about passive runtime boundaries', () => {
    const stalePatterns = [
      /watcher\/refocus\/bootstrap.*Deep Match/i,
      /FileWatcher.*Deep Match Scanner/i,
      /commit_scan.*continuous filesystem sync/i,
      /commit_scan.*passive filesystem projection/i,
      /queryClient\.invalidateQueries/i,
      /raw invalidateQueries/i,
    ];
    const offenders = SCANNER_IMPORT_REQUIREMENT_DOCS.filter((path) =>
      stalePatterns.some((pattern) => pattern.test(readWorkspaceFile(path))),
    );

    expect(offenders).toEqual([]);
  });
});
