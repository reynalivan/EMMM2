import { identityPathKey } from '@/shared/lib/pathKey';

export const thumbnailKeys = {
  all: ['thumbnails'] as const,
  folder: (folderPath: string, gameId?: string) =>
    [
      ...thumbnailKeys.all,
      identityPathKey(folderPath) ?? folderPath,
      ...(gameId ? [gameId] : []),
    ] as const,
};

export const detailsKeys = {
  all: ['details'] as const,
  modInfo: (folderPath: string) => [...detailsKeys.all, 'mod-info', folderPath] as const,
  iniFiles: (folderPath: string) => [...detailsKeys.all, 'ini-files', folderPath] as const,
  iniDocuments: (folderPath: string) => [...detailsKeys.all, 'ini-documents', folderPath] as const,
  iniDocument: (folderPath: string, fileName: string) =>
    [...detailsKeys.all, 'ini-document', folderPath, fileName] as const,
  previewImages: (folderPath: string) =>
    [...detailsKeys.all, 'preview-images', folderPath] as const,
  conflicts: (folderPath: string) => [...detailsKeys.all, 'conflicts', folderPath] as const,
};

export const modHealthKeys = {
  all: ['mod-health'] as const,
  report: (gameId: string, folderPath: string) =>
    [...modHealthKeys.all, 'report', gameId, identityPathKey(folderPath) ?? folderPath] as const,
  viewerSnapshots: () => [...modHealthKeys.all, 'viewer-snapshots'] as const,
  viewerReview: (gameId: string, folderPath: string) =>
    [
      ...modHealthKeys.all,
      'viewer-review',
      gameId,
      identityPathKey(folderPath) ?? folderPath,
    ] as const,
};
