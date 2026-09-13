import { pathStartsWith } from '@/shared/lib/pathKey';

export interface ModViewerManifestFile {
  relative_path: string;
  size_bytes: number;
  content_hash: string;
}

export type ModViewerExternalChangeKind = 'added' | 'modified' | 'removed';

export type ModViewerExternalChangeCategory = 'ini' | 'dds' | 'backup' | 'metadata' | 'other';

export interface ModViewerExternalFileChange {
  kind: ModViewerExternalChangeKind;
  category: ModViewerExternalChangeCategory;
  relative_path: string;
}

function categorize(relativePath: string): ModViewerExternalChangeCategory {
  const path = relativePath.toLowerCase();
  if (path === '.mod_viewer.json') {
    return 'metadata';
  }
  if (path.endsWith('.bak')) {
    return 'backup';
  }
  if (path.endsWith('.ini')) {
    return 'ini';
  }
  if (path.endsWith('.dds')) {
    return 'dds';
  }
  return 'other';
}

function manifestMap(files: readonly ModViewerManifestFile[]): Map<string, ModViewerManifestFile> {
  return new Map(files.map((file) => [file.relative_path.toLowerCase(), file]));
}

function compareChanges(
  left: ModViewerExternalFileChange,
  right: ModViewerExternalFileChange,
): number {
  const kindOrder: Record<ModViewerExternalChangeKind, number> = {
    modified: 0,
    added: 1,
    removed: 2,
  };
  return (
    kindOrder[left.kind] - kindOrder[right.kind] ||
    left.relative_path.localeCompare(right.relative_path, undefined, { sensitivity: 'base' })
  );
}

export function diffModViewerManifests(
  before: readonly ModViewerManifestFile[],
  after: readonly ModViewerManifestFile[],
): ModViewerExternalFileChange[] {
  const beforeFiles = manifestMap(before);
  const afterFiles = manifestMap(after);
  const changes: ModViewerExternalFileChange[] = [];

  for (const [path, current] of afterFiles) {
    const previous = beforeFiles.get(path);
    if (!previous) {
      changes.push({
        kind: 'added',
        category: categorize(current.relative_path),
        relative_path: current.relative_path,
      });
      continue;
    }
    if (
      previous.size_bytes !== current.size_bytes ||
      previous.content_hash !== current.content_hash
    ) {
      changes.push({
        kind: 'modified',
        category: categorize(current.relative_path),
        relative_path: current.relative_path,
      });
    }
  }

  for (const [path, previous] of beforeFiles) {
    if (!afterFiles.has(path)) {
      changes.push({
        kind: 'removed',
        category: categorize(previous.relative_path),
        relative_path: previous.relative_path,
      });
    }
  }

  return changes.sort(compareChanges);
}

function resolveChangedRoot(modsPath: string, changedRoot: string): string {
  return `${modsPath.replace(/[\\/]+$/, '')}/${changedRoot.replace(/^[\\/]+/, '')}`;
}

export function isModViewerSnapshotAffected(
  folderPath: string,
  changedRoots: readonly string[],
  modsPath: string,
): boolean {
  return changedRoots.some((changedRoot) => {
    const absoluteRoot = resolveChangedRoot(modsPath, changedRoot);
    return pathStartsWith(absoluteRoot, folderPath) || pathStartsWith(folderPath, absoluteRoot);
  });
}
