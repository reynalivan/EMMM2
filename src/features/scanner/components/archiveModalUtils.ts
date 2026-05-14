import type { ArchiveInfo } from '../../../types/scanner';
import type { ArchiveGroups, FolderNameValidationMessages } from './archiveModalTypes';

const ILLEGAL_FOLDER_NAME_CHARS = /[<>:"/\\|?*]/;
const WINDOWS_RESERVED_FOLDER_NAME = /^(CON|PRN|AUX|NUL|COM[0-9]|LPT[0-9])$/i;

export function stemName(archiveName: string): string {
  const dot = archiveName.lastIndexOf('.');
  return dot > 0 ? archiveName.slice(0, dot) : archiveName;
}

export function isArchiveEmpty(archive: ArchiveInfo): boolean {
  return archive.file_count === 0 || archive.has_ini === false;
}

export function buildInitialSelectedPaths(archives: ArchiveInfo[]): Set<string> {
  return new Set(
    archives.filter((archive) => !isArchiveEmpty(archive)).map((archive) => archive.path),
  );
}

export function buildInitialFolderNames(archives: ArchiveInfo[]): Record<string, string> {
  const folderNames: Record<string, string> = {};
  for (const archive of archives) {
    folderNames[archive.path] = stemName(archive.name);
  }

  return folderNames;
}

export function groupArchivesByEncryption(archives: ArchiveInfo[]): ArchiveGroups {
  const encrypted: ArchiveInfo[] = [];
  const unencrypted: ArchiveInfo[] = [];

  for (const archive of archives) {
    if (archive.is_encrypted) {
      encrypted.push(archive);
      continue;
    }

    unencrypted.push(archive);
  }

  return { encrypted, unencrypted };
}

export function findDuplicateFolderNames(
  selectedPaths: Set<string>,
  folderNames: Record<string, string>,
): Set<string> {
  const counts = new Map<string, number>();
  for (const path of selectedPaths) {
    const normalizedName = (folderNames[path] ?? '').toLowerCase();
    if (normalizedName.length === 0) {
      continue;
    }

    counts.set(normalizedName, (counts.get(normalizedName) ?? 0) + 1);
  }

  const duplicates = new Set<string>();
  for (const [name, count] of counts) {
    if (count > 1) {
      duplicates.add(name);
    }
  }

  return duplicates;
}

export function buildOverwriteTargets(
  archives: ArchiveInfo[],
  selectedPaths: Set<string>,
  folderNames: Record<string, string>,
  existingFolders: string[],
  autoRename: boolean,
): string[] {
  if (autoRename || existingFolders.length === 0) {
    return [];
  }

  const archiveByPath = new Map(archives.map((archive) => [archive.path, archive]));
  const existingFolderSet = new Set(existingFolders);

  return Array.from(selectedPaths)
    .map((path) => folderNames[path] ?? stemName(archiveByPath.get(path)?.name ?? ''))
    .filter((name) => existingFolderSet.has(name));
}

export function validateFolderName(
  name: string,
  messages: FolderNameValidationMessages,
): string | null {
  const trimmed = name.trim();
  if (trimmed.length === 0) {
    return messages.empty;
  }

  if (ILLEGAL_FOLDER_NAME_CHARS.test(name)) {
    return messages.illegal;
  }

  if (WINDOWS_RESERVED_FOLDER_NAME.test(trimmed)) {
    return messages.reserved;
  }

  return null;
}
