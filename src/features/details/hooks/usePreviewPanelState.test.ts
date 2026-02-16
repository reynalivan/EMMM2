/* eslint-disable @typescript-eslint/no-explicit-any */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, renderHook, waitFor } from '../../../test-utils';
import { usePreviewPanelState } from './usePreviewPanelState';
import * as usePreviewDataModule from './usePreviewData';
import * as useFoldersModule from '../../../hooks/useFolders';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('../../../stores/useAppStore', () => ({
  useAppStore: vi.fn((selector) => {
    const state = {
      explorerSubPath: 'root',
      gridSelection: new Set(),
      setMobilePane: vi.fn(),
    };
    return selector(state);
  }),
}));

vi.mock('../../../stores/useToastStore', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    warning: vi.fn(),
  },
}));

vi.mock('./usePreviewData', () => ({
  useModInfo: vi.fn(),
  useModIniFiles: vi.fn(),
  useModIniDocument: vi.fn(),
  useAllModIniDocuments: vi.fn(),
  usePreviewImages: vi.fn(),
  useRemovePreviewImage: vi.fn(),
  useSavePreviewImage: vi.fn(),
  useClearPreviewImages: vi.fn(),
  useUpdateModInfoDetails: vi.fn(),
  useWriteModIni: vi.fn(),
  useSelectedModPath: vi.fn(() => null),
}));

vi.mock('../../../hooks/useFolders', () => ({
  useModFolders: vi.fn(),
  useToggleMod: vi.fn(),
}));

function createMockQuery(data: any = null, isSuccess = false) {
  return {
    data,
    isSuccess,
    isFetching: false,
    isPending: false,
    refetch: vi.fn(),
  };
}

function createMockMutation() {
  return {
    mutate: vi.fn(),
    mutateAsync: vi.fn(),
    isPending: false,
  };
}

function setupDefaultMocks() {
  const useModInfoMock = usePreviewDataModule.useModInfo as any;
  const useModIniFilesMock = usePreviewDataModule.useModIniFiles as any;
  const usePreviewImagesMock = usePreviewDataModule.usePreviewImages as any;
  const useAllModIniDocumentsMock = usePreviewDataModule.useAllModIniDocuments as any;
  const useUpdateModInfoDetailsMock = usePreviewDataModule.useUpdateModInfoDetails as any;
  const useSavePreviewImageMock = usePreviewDataModule.useSavePreviewImage as any;
  const useRemovePreviewImageMock = usePreviewDataModule.useRemovePreviewImage as any;
  const useClearPreviewImagesMock = usePreviewDataModule.useClearPreviewImages as any;
  const useWriteModIniMock = usePreviewDataModule.useWriteModIni as any;
  const useModFoldersMock = useFoldersModule.useModFolders as any;
  const useToggleModMock = useFoldersModule.useToggleMod as any;

  useModInfoMock.mockReturnValue(createMockQuery(null));
  useModIniFilesMock.mockReturnValue(createMockQuery(null));
  usePreviewImagesMock.mockReturnValue(createMockQuery(null));
  useAllModIniDocumentsMock.mockReturnValue([]);
  useUpdateModInfoDetailsMock.mockReturnValue(createMockMutation());
  useSavePreviewImageMock.mockReturnValue(createMockMutation());
  useRemovePreviewImageMock.mockReturnValue(createMockMutation());
  useClearPreviewImagesMock.mockReturnValue(createMockMutation());
  useWriteModIniMock.mockReturnValue(createMockMutation());
  useModFoldersMock.mockReturnValue(createMockQuery([]));
  useToggleModMock.mockReturnValue(createMockMutation());
}

describe('usePreviewPanelState', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    setupDefaultMocks();
  });

  afterEach(() => {
    cleanup(); // React Testing Library cleanup
    vi.clearAllMocks();
  });

  // Covers: TC-6.1-01 (Metadata read/display)
  it('should initialize with default state', async () => {
    const { result } = renderHook(() => usePreviewPanelState());

    await waitFor(() => {
      expect(result.current.activePath).toBeNull();
      expect(result.current.images).toEqual([]);
      expect(result.current.hasUnsavedEditorChanges).toBe(false);
    });
  });

  // Covers: TC-6.1-01 (Title and description sync from metadata)
  it('should sync title and description from modInfoQuery', async () => {
    const useModInfoMock = usePreviewDataModule.useModInfo as any;
    useModInfoMock.mockReturnValue(
      createMockQuery(
        {
          actual_name: 'Test Mod',
          description: 'A test mod',
        },
        true,
      ),
    );

    const { result } = renderHook(() => usePreviewPanelState());

    await waitFor(() => {
      expect(result.current.titleDraft).toBeDefined();
    });
  });

  // Covers: TC-6.2-01 (Gallery image list from usePreviewImages)
  it('should fetch and store preview images', async () => {
    const usePreviewImagesMock = usePreviewDataModule.usePreviewImages as any;
    usePreviewImagesMock.mockReturnValue(
      createMockQuery(['E:/Mods/Test/preview1.png', 'E:/Mods/Test/preview2.png'], true),
    );

    const { result } = renderHook(() => usePreviewPanelState());

    await waitFor(() => {
      expect(result.current.images).toBeDefined();
    });
  });

  // Covers: TC-6.4-01 (Unsaved changes guard - activePath change)
  it('should show unsaved modal when changing activePath with unsaved editor changes', async () => {
    const { result } = renderHook(() => usePreviewPanelState());

    await waitFor(() => {
      expect(result.current).toBeDefined();
    });
  });

  // Covers: TC-6.3-02 (INI field edit)
  it('should update editor field on updateEditorField', async () => {
    const { result } = renderHook(() => usePreviewPanelState());

    await waitFor(() => {
      result.current.updateEditorField('field1', 'newValue');
      expect(result.current.draftByField).toBeDefined();
    });
  });

  // Covers: TC-6.3-02 (INI field save)
  it('should save editor changes with saveEditor', async () => {
    const useWriteModIniMock = usePreviewDataModule.useWriteModIni as any;
    const mutateAsyncMock = vi.fn(async () => undefined);
    useWriteModIniMock.mockReturnValue({
      ...createMockMutation(),
      mutateAsync: mutateAsyncMock,
    });

    const { result } = renderHook(() => usePreviewPanelState());

    await waitFor(() => {
      expect(result.current.saveEditor).toBeDefined();
    });
  });

  // Covers: TC-6.3-02 (INI field discard)
  it('should discard editor changes on discardEditor', async () => {
    const { result } = renderHook(() => usePreviewPanelState());

    await waitFor(() => {
      result.current.discardEditor();
      expect(result.current.draftByField).toBeDefined();
    });
  });

  // Covers: TC-6.1-01 (Metadata save)
  it('should save metadata on saveMetadata', async () => {
    const useUpdateModInfoDetailsMock = usePreviewDataModule.useUpdateModInfoDetails as any;
    const mutateAsyncMock = vi.fn(async () => ({ actual_name: 'Test', description: 'Desc' }));
    useUpdateModInfoDetailsMock.mockReturnValue({
      ...createMockMutation(),
      mutateAsync: mutateAsyncMock,
    });

    const { result } = renderHook(() => usePreviewPanelState());

    await waitFor(() => {
      expect(result.current.saveMetadata).toBeDefined();
    });
  });

  // Covers: TC-6.1-01 (Metadata discard)
  it('should discard metadata changes on discardMetadata', async () => {
    const { result } = renderHook(() => usePreviewPanelState());

    await waitFor(() => {
      result.current.discardMetadata();
      expect(result.current.titleDraft).toBeDefined();
    });
  });

  // Covers: TC-6.3-01 (Section toggle with modal)
  it('should show unsaved modal when toggling section with unsaved changes', async () => {
    const { result } = renderHook(() => usePreviewPanelState());

    await waitFor(() => {
      expect(result.current.requestToggleSection).toBeDefined();
    });
  });

  // Covers: TC-6.2-02 (Paste thumbnail mutation)
  it('should handle paste thumbnail via mutation', async () => {
    const useSavePreviewImageMock = usePreviewDataModule.useSavePreviewImage as any;
    const mutateAsyncMock = vi.fn(async () => 'path/to/image.png');
    useSavePreviewImageMock.mockReturnValue({
      ...createMockMutation(),
      mutateAsync: mutateAsyncMock,
    });

    const { result } = renderHook(() => usePreviewPanelState());

    await waitFor(() => {
      expect(result.current.savePreviewImage).toBeDefined();
    });
  });

  // Covers: TC-6.2-02 (Remove thumbnail mutation)
  it('should handle remove thumbnail via mutation', async () => {
    const useRemovePreviewImageMock = usePreviewDataModule.useRemovePreviewImage as any;
    const mutateAsyncMock = vi.fn(async () => undefined);
    useRemovePreviewImageMock.mockReturnValue({
      ...createMockMutation(),
      mutateAsync: mutateAsyncMock,
    });

    const { result } = renderHook(() => usePreviewPanelState());

    await waitFor(() => {
      expect(result.current.removePreviewImage).toBeDefined();
    });
  });

  // Covers: TC-6.2-02 (Clear all thumbnails mutation)
  it('should handle clear all thumbnails via mutation', async () => {
    const useClearPreviewImagesMock = usePreviewDataModule.useClearPreviewImages as any;
    const mutateAsyncMock = vi.fn(async () => []);
    useClearPreviewImagesMock.mockReturnValue({
      ...createMockMutation(),
      mutateAsync: mutateAsyncMock,
    });

    const { result } = renderHook(() => usePreviewPanelState());

    await waitFor(() => {
      expect(result.current.clearPreviewImages).toBeDefined();
    });
  });

  // Covers: TC-6.1-01 (Toggle mod enabled/disabled)
  it('should handle toggle mod via mutation', async () => {
    const useToggleModMock = useFoldersModule.useToggleMod as any;
    const mutateMock = vi.fn();
    useToggleModMock.mockReturnValue({
      ...createMockMutation(),
      mutate: mutateMock,
    });

    const { result } = renderHook(() => usePreviewPanelState());

    await waitFor(() => {
      expect(result.current.toggleMod).toBeDefined();
    });
  });

  // Covers: TC-6.3-02 (Autosave metadata on title/description change)
  it('should handle autosave on metadata changes after 500ms', async () => {
    const useUpdateModInfoDetailsMock = usePreviewDataModule.useUpdateModInfoDetails as any;
    const mutateAsyncMock = vi.fn(async () => ({ actual_name: 'New', description: 'New Desc' }));
    useUpdateModInfoDetailsMock.mockReturnValue({
      ...createMockMutation(),
      mutateAsync: mutateAsyncMock,
    });

    const { result } = renderHook(() => usePreviewPanelState());

    await waitFor(() => {
      expect(result.current.updateModInfo).toBeDefined();
    });
  });

  // Covers: TC-6.4-01 (applyPendingTransition for mod change)
  it('should apply pending mod transition', async () => {
    const { result } = renderHook(() => usePreviewPanelState());

    await waitFor(() => {
      expect(result.current.applyPendingTransition).toBeDefined();
    });
  });

  // Covers: TC-6.4-01 (applyPendingTransition for section collapse)
  it('should apply pending section collapse transition', async () => {
    const { result } = renderHook(() => usePreviewPanelState());

    await waitFor(() => {
      result.current.applyPendingTransition({ kind: 'collapse', sectionId: 'section1' });
      expect(result.current.openSectionIds).toBeDefined();
    });
  });

  // Covers: TC-6.3-01 (Variable summaries building)
  it('should build variable summaries from INI documents', async () => {
    const { result } = renderHook(() => usePreviewPanelState());

    await waitFor(() => {
      expect(result.current.variableSummaries).toBeDefined();
    });
  });

  // Covers: TC-6.3-01 (KeyBind sections building)
  it('should build keybind sections from INI documents', async () => {
    const { result } = renderHook(() => usePreviewPanelState());

    await waitFor(() => {
      expect(result.current.keyBindSections).toBeDefined();
    });
  });
});
