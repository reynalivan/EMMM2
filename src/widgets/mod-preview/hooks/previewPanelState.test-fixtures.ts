import { vi } from 'vitest';
import { MutationObserver, QueryClient, QueryObserver } from '@tanstack/react-query';

const selectionOverride = vi.hoisted(() => ({ path: undefined as string | null | undefined }));

export function selectionOverrideState() {
  return selectionOverride;
}

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('@/app/store', () => {
  // One shared state object with the workspace runtime slice, so `getState()`
  // and the selector hook see the same store the app does.
  const state = {
    explorerSubPath: 'root',
    gridSelection: new Set(),
    setMobilePane: vi.fn(),
    selectedObjectFolderPath: null,
    selectedModPath: null,
    currentPath: [] as string[],
    mobileActivePane: 'sidebar' as const,
    workspacePreviewDirty: false,
    workspacePreviewTransition: { kind: 'idle', pendingTarget: null },
    workspaceDialogState: { kind: 'none' },
    dispatchWorkspaceRuntime: vi.fn(),
    activeGameId: 'GIMI',
    folderConflictsByGame: {} as Record<string, unknown[]>,
  };

  return {
    useAppStore: Object.assign(
      vi.fn((selector) => (typeof selector === 'function' ? selector(state) : state)),
      { getState: () => state },
    ),
  };
});

vi.mock('@/shared/ui/toast', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    warning: vi.fn(),
  },
}));

const dataMocks = vi.hoisted(() => ({
  useModIniDocuments: vi.fn<(...args: unknown[]) => unknown>(),
  usePreviewImages: vi.fn<(...args: unknown[]) => unknown>(),
  useRemovePreviewImage: vi.fn<(...args: unknown[]) => unknown>(),
  useSavePreviewImage: vi.fn<(...args: unknown[]) => unknown>(),
  useClearPreviewImages: vi.fn<(...args: unknown[]) => unknown>(),
  useUpdateModInfoDetails: vi.fn<(...args: unknown[]) => unknown>(),
  useWriteModIni: vi.fn<(...args: unknown[]) => unknown>(),
}));
const workspaceQuery = vi.hoisted(() =>
  vi.fn<
    () => {
      data?: { preview?: Record<string, unknown>; runtime?: unknown } | null;
      isPending?: boolean;
    }
  >(),
);
export function previewDataMocks() {
  return dataMocks;
}
export function workspaceQueryMock() {
  return workspaceQuery;
}
vi.mock('./usePreviewData', () => ({ ...dataMocks, useSelectedModPath: vi.fn(() => null) }));

vi.mock('@/features/workspace-runtime/hooks/useWorkspaceViewModel', () => {
  const useWorkspaceViewModel = workspaceQuery;
  workspaceQuery.mockReturnValue({
    isPending: false,
    data: {
      runtime: {
        source_state: { status: 'available', message: null },
      },
      preview: {
        selected_path: null,
        selected_node: null,
        is_flat_mod_root: false,
        display_title: null,
        display_subtitle: null,
        mod_info_summary: null,
        ini_summary: null,
        image_summary: null,
        warning_summary: {
          state: 'none',
          messages: [],
        },
      },
    },
  });

  return {
    useWorkspaceViewModel,
    useWorkspaceStructure: () => {
      const result = useWorkspaceViewModel();
      return {
        ...result,
        isPlaceholderData: false,
        data: result.data
          ? {
              ...result.data,
            }
          : result.data,
      };
    },
    useWorkspacePreview: () => {
      const result = useWorkspaceViewModel();
      const preview = result.data?.preview;
      return {
        data:
          preview && !result.isPending
            ? {
                request_identity: {
                  game_id: 'GIMI',
                  explorer_sub_path: 'root',
                  selected_mod_path: preview.selected_path,
                },
                context_status: 'ready',
                preview,
                selection: {
                  selected_mod_path: preview.selected_path,
                  reconciliation_status: 'unchanged',
                  reconciliation_reason: null,
                  affected_paths: [],
                },
              }
            : undefined,
        isPending: false,
        isError: false,
        error: null,
        refetch: vi.fn(),
      };
    },
    useWorkspaceSelectionInput: () => {
      const result = useWorkspaceViewModel();
      return {
        selectedObjectFolderPath: null,
        explorerSubPath: 'root',
        selectedModPath:
          selectionOverride.path === undefined
            ? (result.data?.preview?.selected_path ?? null)
            : selectionOverride.path,
      };
    },
  };
});

export function createMockQuery<T>(data: T | null = null, isSuccess = false) {
  return {
    ...new QueryObserver(new QueryClient(), {
      queryKey: ['preview-test'],
      initialData: data,
      enabled: false,
    }).getCurrentResult(),
    data,
    isSuccess,
    isFetching: false,
    isPending: false,
    refetch: vi.fn(),
  };
}

export function createMockMutation() {
  return {
    ...new MutationObserver(new QueryClient(), {
      mutationFn: async () => undefined,
    }).getCurrentResult(),
    mutate: vi.fn(),
    mutateAsync: vi.fn(),
    isPending: false,
  };
}

export function setupDefaultMocks() {
  selectionOverride.path = undefined;
  const useModIniDocumentsMock = dataMocks.useModIniDocuments;
  const usePreviewImagesMock = dataMocks.usePreviewImages;
  const useUpdateModInfoDetailsMock = dataMocks.useUpdateModInfoDetails;
  const useSavePreviewImageMock = dataMocks.useSavePreviewImage;
  const useRemovePreviewImageMock = dataMocks.useRemovePreviewImage;
  const useClearPreviewImagesMock = dataMocks.useClearPreviewImages;
  const useWriteModIniMock = dataMocks.useWriteModIni;
  const useWorkspaceViewModelMock = workspaceQuery;

  useModIniDocumentsMock.mockReturnValue(createMockQuery(null));
  usePreviewImagesMock.mockReturnValue(createMockQuery(null));
  useUpdateModInfoDetailsMock.mockReturnValue(createMockMutation());
  useSavePreviewImageMock.mockReturnValue(createMockMutation());
  useRemovePreviewImageMock.mockReturnValue(createMockMutation());
  useClearPreviewImagesMock.mockReturnValue(createMockMutation());
  useWriteModIniMock.mockReturnValue(createMockMutation());
  useWorkspaceViewModelMock.mockReturnValue({
    data: {
      preview: {
        selected_path: null,
        selected_node: null,
        is_flat_mod_root: false,
        display_title: null,
        display_subtitle: null,
        mod_info_summary: null,
        ini_summary: null,
        image_summary: null,
        warning_summary: {
          state: 'none',
          messages: [],
        },
      },
    },
  });
}
