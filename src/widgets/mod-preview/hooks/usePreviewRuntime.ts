import { useMemo } from 'react';
import { useWorkspaceViewModel } from '@/features/workspace-runtime/hooks/useWorkspaceViewModel';
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
} from '@/entities/workspace/model/workspace';
import type { IniDocumentLike } from '../utils/previewPanelUtils';
import { DEFAULT_SOURCE_UNAVAILABLE_MESSAGE } from '@/features/workspace-runtime/actions/workspaceActionAvailability';
import { useAppStore } from '../../../app/store/useAppStore';
import { isFolderConflictProtected } from '@/widgets/mod-explorer/hooks/folderConflictScope';
import type { FolderNameConflictGroup } from '../../../shared/api/tauri/bindings';

interface PreviewIniDocument {
  fileName: string;
  document: IniDocumentLike | null | undefined;
}

interface PreviewRuntimeState {
  activePath: string | null;
  folderNameConflict: FolderNameConflictGroup | null;
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
  const activeGameId = useAppStore((state) => state.activeGameId);
  const folderConflicts = useAppStore((state) =>
    activeGameId ? (state.folderConflictsByGame[activeGameId] ?? []) : [],
  );
  const folderNameConflict =
    activePath === null
      ? null
      : (folderConflicts.find((group) =>
          group.candidates.some((candidate) =>
            isFolderConflictProtected(activePath, [candidate.path]),
          ),
        ) ?? null);
  const detailPath = folderNameConflict ? null : activePath;
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

  const iniFilesQuery = useModIniFiles(detailPath);
  const previewImagesQuery = usePreviewImages(detailPath);
  // Each mutation invalidates its own detail queries in `usePreviewData`.
  const updateModInfo = useUpdateModInfoDetails();
  const savePreviewImage = useSavePreviewImage();
  const removePreviewImage = useRemovePreviewImage();
  const clearPreviewImages = useClearPreviewImages();
  const writeModIni = useWriteModIni();

  const iniFiles = useMemo<IniFileEntry[]>(() => iniFilesQuery.data ?? [], [iniFilesQuery.data]);

  const allIniQueries = useAllModIniDocuments(detailPath, iniFiles);
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
    folderNameConflict,
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
