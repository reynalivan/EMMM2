import { useMemo } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useWorkspaceViewModel } from '../../workspace-runtime/useWorkspaceViewModel';
import {
  detailsKeys,
  useAllModIniDocuments,
  useClearPreviewImages,
  useModIniFiles,
  usePreviewImages,
  useRemovePreviewImage,
  useSavePreviewImage,
  useUpdateModInfoDetails,
  useWriteModIni,
  type IniFileEntry,
} from './usePreviewData';
import {
  isWorkspaceExplorerNode,
  type WorkspaceExplorerNode,
  type WorkspacePreview,
  type WorkspaceViewModel,
} from '../../../types/workspace';
import type { IniDocumentLike } from '../previewPanelUtils';
import { applyRuntimeEffects } from '../../workspace-runtime/optimistic/applyOptimisticEffects';
import { buildQueryInvalidationDescriptor } from '../../workspace-runtime/optimistic/descriptorBuilders';

interface PreviewIniDocument {
  fileName: string;
  document: IniDocumentLike | null | undefined;
}

interface PreviewRuntimeState {
  activePath: string | null;
  selectedFolder: WorkspaceExplorerNode | null;
  previewSummary: WorkspacePreview | null;
  resolvedTitle: string | null;
  resolvedSubtitle: string | null;
  availableObjects: WorkspaceViewModel['objects'];
  iniFiles: IniFileEntry[];
  iniDocuments: PreviewIniDocument[];
  images: string[];
  previewImagesQuery: ReturnType<typeof usePreviewImages>;
  updateModInfo: ReturnType<typeof useUpdateModInfoDetails>;
  savePreviewImage: ReturnType<typeof useSavePreviewImage>;
  removePreviewImage: ReturnType<typeof useRemovePreviewImage>;
  clearPreviewImages: ReturnType<typeof useClearPreviewImages>;
  writeModIni: ReturnType<typeof useWriteModIni>;
}

export function usePreviewRuntime(): PreviewRuntimeState {
  const queryClient = useQueryClient();
  const { data: workspace } = useWorkspaceViewModel();
  const activePath = workspace?.preview.selected_path ?? null;
  const previewSummary = workspace?.preview ?? null;
  const selectedNode = workspace?.preview.selected_node ?? null;
  const selectedFolder = isWorkspaceExplorerNode(selectedNode) ? selectedNode : null;
  const availableObjects = workspace?.objects ?? [];
  const resolvedTitle = workspace?.preview.display_title ?? selectedFolder?.display_name ?? null;
  const resolvedSubtitle = workspace?.preview.display_subtitle ?? null;

  const iniFilesQuery = useModIniFiles(activePath);
  const previewImagesQuery = usePreviewImages(activePath);
  const updateModInfoRaw = useUpdateModInfoDetails();
  const savePreviewImageRaw = useSavePreviewImage();
  const removePreviewImageRaw = useRemovePreviewImage();
  const clearPreviewImagesRaw = useClearPreviewImages();
  const writeModIniRaw = useWriteModIni();

  const iniFiles = useMemo(
    () =>
      (iniFilesQuery.data ?? []).map((filename) =>
        typeof filename === 'string' ? { filename, path: filename } : filename,
      ) as IniFileEntry[],
    [iniFilesQuery.data],
  );

  const allIniQueries = useAllModIniDocuments(activePath, iniFiles);
  const iniDocuments = useMemo(
    () =>
      iniFiles.map((file, index) => ({
        fileName: file.filename,
        document: allIniQueries[index]?.data as IniDocumentLike | null | undefined,
      })),
    [allIniQueries, iniFiles],
  );

  const images = useMemo(() => previewImagesQuery.data ?? [], [previewImagesQuery.data]);

  const updateModInfo = useMemo(
    () => ({
      ...updateModInfoRaw,
      mutateAsync: async (input: Parameters<typeof updateModInfoRaw.mutateAsync>[0]) => {
        const result = await updateModInfoRaw.mutateAsync(input);
        applyRuntimeEffects(
          queryClient,
          buildQueryInvalidationDescriptor([detailsKeys.modInfo(input.folderPath)], []),
        );
        return result;
      },
    }),
    [queryClient, updateModInfoRaw],
  );

  const savePreviewImage = useMemo(
    () => ({
      ...savePreviewImageRaw,
      mutateAsync: async (input: Parameters<typeof savePreviewImageRaw.mutateAsync>[0]) => {
        const result = await savePreviewImageRaw.mutateAsync(input);
        applyRuntimeEffects(
          queryClient,
          buildQueryInvalidationDescriptor([detailsKeys.previewImages(input.folderPath)], []),
        );
        return result;
      },
    }),
    [queryClient, savePreviewImageRaw],
  );

  const removePreviewImage = useMemo(
    () => ({
      ...removePreviewImageRaw,
      mutateAsync: async (input: Parameters<typeof removePreviewImageRaw.mutateAsync>[0]) => {
        const result = await removePreviewImageRaw.mutateAsync(input);
        applyRuntimeEffects(
          queryClient,
          buildQueryInvalidationDescriptor([detailsKeys.previewImages(input.folderPath)], []),
        );
        return result;
      },
    }),
    [queryClient, removePreviewImageRaw],
  );

  const clearPreviewImages = useMemo(
    () => ({
      ...clearPreviewImagesRaw,
      mutateAsync: async (input: Parameters<typeof clearPreviewImagesRaw.mutateAsync>[0]) => {
        const result = await clearPreviewImagesRaw.mutateAsync(input);
        applyRuntimeEffects(
          queryClient,
          buildQueryInvalidationDescriptor([detailsKeys.previewImages(input.folderPath)], []),
        );
        return result;
      },
    }),
    [queryClient, clearPreviewImagesRaw],
  );

  const writeModIni = useMemo(
    () => ({
      ...writeModIniRaw,
      mutateAsync: async (input: Parameters<typeof writeModIniRaw.mutateAsync>[0]) => {
        const result = await writeModIniRaw.mutateAsync(input);
        applyRuntimeEffects(
          queryClient,
          buildQueryInvalidationDescriptor(
            [
              detailsKeys.iniDocument(input.folderPath, input.fileName),
              detailsKeys.iniFiles(input.folderPath),
            ],
            [],
          ),
        );
        return result;
      },
    }),
    [queryClient, writeModIniRaw],
  );

  return {
    activePath,
    selectedFolder,
    previewSummary,
    resolvedTitle,
    resolvedSubtitle,
    availableObjects,
    iniFiles,
    iniDocuments,
    images,
    previewImagesQuery,
    updateModInfo,
    savePreviewImage,
    removePreviewImage,
    clearPreviewImages,
    writeModIni,
  };
}
