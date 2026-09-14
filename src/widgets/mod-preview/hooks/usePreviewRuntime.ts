import { useMemo } from 'react';
import {
  useWorkspacePreview,
  useWorkspaceSelectionInput,
  useWorkspaceStructure,
} from '@/features/workspace-runtime';
import {
  useClearPreviewImages,
  useModIniDocuments,
  usePreviewImages,
  useRemovePreviewImage,
  useSavePreviewImage,
  useUpdateModInfoDetails,
  useWriteModIni,
} from './usePreviewData';
import {
  isWorkspaceExplorerNode,
  type WorkspaceExplorerNode,
  type WorkspacePreview,
  type WorkspaceStructureViewModel,
} from '@/entities/workspace';
import type { IniDocumentLike } from '../utils/previewPanelUtils';
import { DEFAULT_SOURCE_UNAVAILABLE_MESSAGE } from '@/features/workspace-runtime';
import { useAppStore } from '@/app/store';
import { isFolderConflictProtected } from '@/features/workspace-runtime';
import type { FolderNameConflictGroup } from '../../../shared/api/tauri/bindings';

const EMPTY_FOLDER_CONFLICTS: FolderNameConflictGroup[] = [];

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
  availableObjects: WorkspaceStructureViewModel['objects'];
  isPreviewLoading: boolean;
  previewError: unknown | null;
  retryPreview: () => Promise<unknown>;
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
  const structureQuery = useWorkspaceStructure();
  const workspace = structureQuery.data;
  const currentSelection = useWorkspaceSelectionInput();
  const sourceUnavailableMessage =
    workspace?.runtime?.source_state?.status === 'unavailable'
      ? (workspace.runtime.source_state.message ?? DEFAULT_SOURCE_UNAVAILABLE_MESSAGE)
      : null;
  const previewQuery = useWorkspacePreview({
    enabled: Boolean(workspace) && !sourceUnavailableMessage && !structureQuery.isPlaceholderData,
  });
  const previewResult = previewQuery.data?.context_status === 'ready' ? previewQuery.data : null;
  const activePath = previewResult?.preview.selected_path ?? null;
  const activeGameId = useAppStore((state) => state.activeGameId);
  const folderConflicts = useAppStore((state) =>
    activeGameId
      ? (state.folderConflictsByGame[activeGameId] ?? EMPTY_FOLDER_CONFLICTS)
      : EMPTY_FOLDER_CONFLICTS,
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
  const previewSummary = previewResult?.preview ?? null;
  const selectedNode = previewResult?.preview.selected_node ?? null;
  const selectedFolder = isWorkspaceExplorerNode(selectedNode) ? selectedNode : null;
  const availableObjects = workspace?.objects ?? [];
  const resolvedTitle = previewSummary?.display_title ?? selectedFolder?.display_name ?? null;
  const resolvedSubtitle = previewSummary?.display_subtitle ?? null;
  const isPreviewLoading = Boolean(
    currentSelection.selectedModPath &&
    !sourceUnavailableMessage &&
    (!workspace || previewQuery.isPending || previewQuery.data?.context_status === 'context_stale'),
  );
  const previewError = previewQuery.isError ? previewQuery.error : null;

  const iniDocumentsQuery = useModIniDocuments(detailPath);
  const previewImagesQuery = usePreviewImages(detailPath);
  // Each mutation invalidates its own detail queries in `usePreviewData`.
  const updateModInfo = useUpdateModInfoDetails();
  const savePreviewImage = useSavePreviewImage();
  const removePreviewImage = useRemovePreviewImage();
  const clearPreviewImages = useClearPreviewImages();
  const writeModIni = useWriteModIni();

  const iniDocuments = useMemo(
    () =>
      (iniDocumentsQuery.data ?? []).map((entry) => ({
        fileName: entry.filename,
        document: entry.document as IniDocumentLike,
      })),
    [iniDocumentsQuery.data],
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
    isPreviewLoading,
    previewError,
    retryPreview: previewQuery.refetch,
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
