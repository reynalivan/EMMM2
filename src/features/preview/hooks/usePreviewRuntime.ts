import { useMemo } from 'react';
import { useWorkspaceViewModel } from '../../workspace-runtime/useWorkspaceViewModel';
import {
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
import { DEFAULT_SOURCE_UNAVAILABLE_MESSAGE } from '../../workspace-runtime/actions/workspaceActionAvailability';

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
  sourceUnavailableMessage: string | null;
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
  const { data: workspace } = useWorkspaceViewModel();
  const activePath = workspace?.preview.selected_path ?? null;
  const previewSummary = workspace?.preview ?? null;
  const selectedNode = workspace?.preview.selected_node ?? null;
  const selectedFolder = isWorkspaceExplorerNode(selectedNode) ? selectedNode : null;
  const availableObjects = workspace?.objects ?? [];
  const sourceUnavailableMessage =
    workspace?.runtime?.source_state?.status === 'unavailable'
      ? (workspace.runtime.source_state.message ?? DEFAULT_SOURCE_UNAVAILABLE_MESSAGE)
      : null;
  const resolvedTitle = workspace?.preview.display_title ?? selectedFolder?.display_name ?? null;
  const resolvedSubtitle = workspace?.preview.display_subtitle ?? null;

  const iniFilesQuery = useModIniFiles(activePath);
  const previewImagesQuery = usePreviewImages(activePath);
  // Each mutation invalidates its own detail queries in `usePreviewData`.
  const updateModInfo = useUpdateModInfoDetails();
  const savePreviewImage = useSavePreviewImage();
  const removePreviewImage = useRemovePreviewImage();
  const clearPreviewImages = useClearPreviewImages();
  const writeModIni = useWriteModIni();

  const iniFiles = useMemo<IniFileEntry[]>(() => iniFilesQuery.data ?? [], [iniFilesQuery.data]);

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

  return {
    activePath,
    selectedFolder,
    previewSummary,
    resolvedTitle,
    resolvedSubtitle,
    sourceUnavailableMessage,
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
