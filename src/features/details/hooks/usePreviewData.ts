import { useMemo } from 'react';
import { useMutation, useQueries, useQuery, useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import type { ModInfo, ModInfoUpdate } from '../../../types/mod';
import { folderKeys } from '../../../hooks/useFolders';
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

export interface IniDocument {
  file_path: string;
  raw_lines: string[];
  variables: IniVariable[];
  key_bindings: KeyBinding[];
  had_bom: boolean;
  newline_style: NewlineStyle;
  mode: IniReadMode;
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

export interface PastePreviewImageInput {
  folderPath: string;
  imageData: number[];
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
};

function normalizeFolderPath(folderPath?: string | null): string | null {
  const value = folderPath?.trim();
  return value ? value : null;
}

function normalizeFileName(fileName?: string | null): string | null {
  const value = fileName?.trim();
  return value ? value : null;
}

export function useModInfo(folderPath?: string | null) {
  const normalizedPath = normalizeFolderPath(folderPath);

  return useQuery({
    queryKey: detailsKeys.modInfo(normalizedPath ?? ''),
    queryFn: () => invoke<ModInfo | null>('read_mod_info', { folderPath: normalizedPath }),
    enabled: !!normalizedPath,
    staleTime: 10_000,
  });
}

export function useModIniFiles(folderPath?: string | null) {
  const normalizedPath = normalizeFolderPath(folderPath);

  return useQuery({
    queryKey: detailsKeys.iniFiles(normalizedPath ?? ''),
    queryFn: () => invoke<IniFileEntry[]>('list_mod_ini_files', { folderPath: normalizedPath }),
    enabled: !!normalizedPath,
    staleTime: 10_000,
  });
}

export function useModIniDocument(folderPath?: string | null, fileName?: string | null) {
  const normalizedPath = normalizeFolderPath(folderPath);
  const normalizedName = normalizeFileName(fileName);

  return useQuery({
    queryKey: detailsKeys.iniDocument(normalizedPath ?? '', normalizedName ?? ''),
    queryFn: () =>
      invoke<IniDocument>('read_mod_ini', {
        folderPath: normalizedPath,
        fileName: normalizedName,
      }),
    enabled: !!normalizedPath && !!normalizedName,
    staleTime: 0,
  });
}

export function useAllModIniDocuments(folderPath?: string | null, files?: IniFileEntry[]) {
  const normalizedPath = normalizeFolderPath(folderPath);
  const safeFiles = files ?? [];

  return useQueries({
    queries: safeFiles.map((file) => ({
      queryKey: detailsKeys.iniDocument(normalizedPath ?? '', file.filename),
      queryFn: () =>
        invoke<IniDocument>('read_mod_ini', {
          folderPath: normalizedPath,
          fileName: file.filename,
        }),
      enabled: !!normalizedPath,
      staleTime: 0,
    })),
  });
}

export function usePreviewImages(folderPath?: string | null) {
  const normalizedPath = normalizeFolderPath(folderPath);

  return useQuery({
    queryKey: detailsKeys.previewImages(normalizedPath ?? ''),
    queryFn: () => invoke<string[]>('list_mod_preview_images', { folderPath: normalizedPath }),
    enabled: !!normalizedPath,
    staleTime: 10_000,
  });
}

export function useWriteModIni() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: WriteModIniInput) => invoke<void>('write_mod_ini', { ...input }),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({
        queryKey: detailsKeys.iniDocument(variables.folderPath, variables.fileName),
      });
      queryClient.invalidateQueries({ queryKey: detailsKeys.iniFiles(variables.folderPath) });
    },
  });
}

export function usePastePreviewImage() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: PastePreviewImageInput) => invoke<string>('paste_thumbnail', { ...input }),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: detailsKeys.previewImages(variables.folderPath) });
      queryClient.invalidateQueries({ queryKey: folderKeys.all });
    },
  });
}

export function useSavePreviewImage() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: SavePreviewImageInput) =>
      invoke<string>('save_mod_preview_image', { ...input }),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: detailsKeys.previewImages(variables.folderPath) });
      queryClient.invalidateQueries({ queryKey: folderKeys.all });
    },
  });
}

export function useRemovePreviewImage() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: RemovePreviewImageInput) =>
      invoke<void>('remove_mod_preview_image', { ...input }),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: detailsKeys.previewImages(variables.folderPath) });
      queryClient.invalidateQueries({ queryKey: folderKeys.all });
    },
  });
}

export function useClearPreviewImages() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: ClearPreviewImagesInput) =>
      invoke<string[]>('clear_mod_preview_images', { ...input }),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: detailsKeys.previewImages(variables.folderPath) });
      queryClient.invalidateQueries({ queryKey: folderKeys.all });
    },
  });
}

export function useUpdateModInfoDetails() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: UpdateModInfoInput) => invoke<ModInfo>('update_mod_info', { ...input }),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: detailsKeys.modInfo(variables.folderPath) });
      queryClient.invalidateQueries({ queryKey: folderKeys.all });
    },
  });
}

export function useSelectedModPath() {
  const gridSelection = useAppStore((state) => state.gridSelection);

  return useMemo(() => {
    let lastSelected: string | null = null;
    for (const path of gridSelection) {
      lastSelected = path;
    }
    return lastSelected;
  }, [gridSelection]);
}

export function useSelectedModInfo() {
  const selectedPath = useSelectedModPath();
  return useModInfo(selectedPath);
}

export function useSelectedModIniFiles() {
  const selectedPath = useSelectedModPath();
  return useModIniFiles(selectedPath);
}

export function useSelectedPreviewImages() {
  const selectedPath = useSelectedModPath();
  return usePreviewImages(selectedPath);
}
