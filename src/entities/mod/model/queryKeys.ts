import { identityPathKey } from '@/shared/lib/pathKey';

export const thumbnailKeys = {
  all: ['thumbnails'] as const,
  folder: (folderPath: string) =>
    [...thumbnailKeys.all, identityPathKey(folderPath) ?? folderPath] as const,
};

export const detailsKeys = {
  all: ['details'] as const,
  modInfo: (folderPath: string) => [...detailsKeys.all, 'mod-info', folderPath] as const,
  iniFiles: (folderPath: string) => [...detailsKeys.all, 'ini-files', folderPath] as const,
  iniDocument: (folderPath: string, fileName: string) =>
    [...detailsKeys.all, 'ini-document', folderPath, fileName] as const,
  previewImages: (folderPath: string) =>
    [...detailsKeys.all, 'preview-images', folderPath] as const,
  conflicts: (folderPath: string) => [...detailsKeys.all, 'conflicts', folderPath] as const,
};
