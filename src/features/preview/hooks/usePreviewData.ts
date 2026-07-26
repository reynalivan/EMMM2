import { useMutation, useQueries, useQuery } from '@tanstack/react-query';
import type { ModInfoUpdate } from '../../../types/mod';
import { commands, sparse } from '../../../lib/bindings';
import { useAppStore } from '../../../stores/useAppStore';

export interface IniFileEntry {
  filename: string;
  path: string;
}

export type IniReadMode = 'Structured' | 'RawFallback';

export type NewlineStyle = 'Lf' | 'CrLf';

export interface IniVariable {
  name: string;
  value: string;
  line_idx: number;
}

export interface KeyBinding {
  section_name: string;
  key: string | null;
  back: string | null;
  key_line_idx: number | null;
  back_line_idx: number | null;
}

export interface IniLineUpdate {
  line_idx: number;
  content: string;
}

export interface WriteModIniInput {
  folderPath: string;
  fileName: string;
  lineUpdates: IniLineUpdate[];
}

export interface SavePreviewImageInput {
  folderPath: string;
  objectName: string;
  imageData: number[];
}

export interface RemovePreviewImageInput {
  folderPath: string;
  imagePath: string;
}

export interface ClearPreviewImagesInput {
  folderPath: string;
}

export interface UpdateModInfoInput {
  folderPath: string;
  update: ModInfoUpdate;
}

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

function normalizeFolderPath(folderPath?: string | null): string | null {
  const value = folderPath?.trim();
  return value ? value : null;
}

function normalizeFileName(fileName?: string | null): string | null {
  const value = fileName?.trim();
  return value ? value : null;
}

/**
 * Every preview command is scoped to the active game on the Rust side. The panel
 * only ever renders mods from that game, so resolve it here instead of threading
 * `gameId` through each hook signature.
 */
function useActiveGameId(): string {
  return useAppStore((state) => state.activeGameId) ?? '';
}

export function useModInfo(folderPath?: string | null) {
  const normalizedPath = normalizeFolderPath(folderPath);
  const gameId = useActiveGameId();

  return useQuery({
    queryKey: detailsKeys.modInfo(normalizedPath ?? ''),
    queryFn: () => commands.readModInfo(gameId, normalizedPath ?? ''),
    enabled: !!normalizedPath && !!gameId,
    staleTime: 10_000,
  });
}

export function useModIniFiles(folderPath?: string | null) {
  const normalizedPath = normalizeFolderPath(folderPath);
  const gameId = useActiveGameId();

  return useQuery({
    queryKey: detailsKeys.iniFiles(normalizedPath ?? ''),
    queryFn: () => commands.listModIniFiles(gameId, normalizedPath ?? ''),
    enabled: !!normalizedPath && !!gameId,
    staleTime: 10_000,
  });
}

export function useModIniDocument(folderPath?: string | null, fileName?: string | null) {
  const normalizedPath = normalizeFolderPath(folderPath);
  const normalizedName = normalizeFileName(fileName);
  const gameId = useActiveGameId();

  return useQuery({
    queryKey: detailsKeys.iniDocument(normalizedPath ?? '', normalizedName ?? ''),
    queryFn: () => commands.readModIni(gameId, normalizedPath ?? '', normalizedName ?? ''),
    enabled: !!normalizedPath && !!normalizedName && !!gameId,
    staleTime: 0,
  });
}

export function useAllModIniDocuments(folderPath?: string | null, files?: IniFileEntry[]) {
  const normalizedPath = normalizeFolderPath(folderPath);
  const safeFiles = files ?? [];
  const gameId = useActiveGameId();

  return useQueries({
    queries: safeFiles.map((file) => ({
      queryKey: detailsKeys.iniDocument(normalizedPath ?? '', file.filename),
      queryFn: () => commands.readModIni(gameId, normalizedPath ?? '', file.filename),
      enabled: !!normalizedPath && !!gameId,
      staleTime: 0,
    })),
  });
}

export function usePreviewImages(folderPath?: string | null) {
  const normalizedPath = normalizeFolderPath(folderPath);
  const gameId = useActiveGameId();

  return useQuery({
    queryKey: detailsKeys.previewImages(normalizedPath ?? ''),
    queryFn: () => commands.listModPreviewImages(gameId, normalizedPath ?? ''),
    enabled: !!normalizedPath && !!gameId,
    staleTime: 10_000,
  });
}

export function useWriteModIni() {
  const gameId = useActiveGameId();
  return useMutation({
    mutationFn: (input: WriteModIniInput) =>
      commands.writeModIni(gameId, input.folderPath, input.fileName, input.lineUpdates),
  });
}

export function useSavePreviewImage() {
  const gameId = useActiveGameId();
  return useMutation({
    mutationFn: (input: SavePreviewImageInput) =>
      commands.saveModPreviewImage(gameId, input.folderPath, input.objectName, input.imageData),
  });
}

export function useRemovePreviewImage() {
  const gameId = useActiveGameId();
  return useMutation({
    mutationFn: (input: RemovePreviewImageInput) =>
      commands.removeModPreviewImage(gameId, input.folderPath, input.imagePath),
  });
}

export function useClearPreviewImages() {
  const gameId = useActiveGameId();
  return useMutation({
    mutationFn: (input: ClearPreviewImagesInput) =>
      commands.clearModPreviewImages(gameId, input.folderPath),
  });
}

export function useUpdateModInfoDetails() {
  const gameId = useActiveGameId();
  return useMutation({
    mutationFn: (input: UpdateModInfoInput) =>
      commands.updateModInfo(gameId, input.folderPath, sparse(input.update)),
  });
}
