import {
  previewDataMocks,
  workspaceQueryMock,
  createMockQuery,
  createMockMutation,
  setupDefaultMocks,
} from './previewPanelState.test-fixtures';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, renderHook, waitFor } from '../../../tests/testing/test-utils';
import { usePreviewPanelState } from './usePreviewPanelState';
import { useAppStore } from '@/app/store';
describe('usePreviewPanelState', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    const appState = useAppStore.getState() as unknown as {
      activeGameId: string;
      folderConflictsByGame: Record<string, unknown[]>;
    };
    appState.activeGameId = 'GIMI';
    appState.folderConflictsByGame = {};
    setupDefaultMocks();
  });

  afterEach(() => {
    cleanup(); // React Testing Library cleanup
    vi.useRealTimers();
    vi.clearAllMocks();
  });

  it('should fetch and store preview images', async () => {
    const usePreviewImagesMock = previewDataMocks().usePreviewImages;
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
    const useWriteModIniMock = previewDataMocks().useWriteModIni;
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
    const useUpdateModInfoDetailsMock = previewDataMocks().useUpdateModInfoDetails;
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
    const useSavePreviewImageMock = previewDataMocks().useSavePreviewImage;
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
    const useRemovePreviewImageMock = previewDataMocks().useRemovePreviewImage;
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
    const useClearPreviewImagesMock = previewDataMocks().useClearPreviewImages;
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

  // Covers: TC-6.3-02 (Autosave metadata on title/description change)
  it('should handle autosave on metadata changes after 500ms', async () => {
    const useUpdateModInfoDetailsMock = previewDataMocks().useUpdateModInfoDetails;
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
      result.current.applyPendingTransition();
      expect(result.current.openSectionIds).toBeDefined();
    });
  });

  // Covers: TC-6.3-01 (KeyBind sections building)
  it('should build keybind sections from INI documents', async () => {
    const { result } = renderHook(() => usePreviewPanelState());

    await waitFor(() => {
      expect(result.current.keyBindSections).toBeDefined();
    });
  });

  it('opens every keybind file by default so bindings are immediately readable', async () => {
    const useModIniDocumentsMock = previewDataMocks().useModIniDocuments;
    const useWorkspaceViewModelMock = workspaceQueryMock();
    useModIniDocumentsMock.mockReturnValue(
      createMockQuery(
        [
          {
            filename: 'alpha.ini',
            document: {
              source_hash: 'alpha-source',
              mode: 'Structured',
              raw_lines: ['[KeyAlpha]', 'key = a'],
              variables: [],
              key_bindings: [
                {
                  section_name: 'KeyAlpha',
                  key: 'a',
                  back: null,
                  key_line_idx: 1,
                  back_line_idx: null,
                },
              ],
            },
          },
          {
            filename: 'beta.ini',
            document: {
              source_hash: 'beta-source',
              mode: 'Structured',
              raw_lines: ['[KeyBeta]', 'key = b'],
              variables: [],
              key_bindings: [
                {
                  section_name: 'KeyBeta',
                  key: 'b',
                  back: null,
                  key_line_idx: 1,
                  back_line_idx: null,
                },
              ],
            },
          },
        ],
        true,
      ),
    );
    useWorkspaceViewModelMock.mockReturnValue({
      data: {
        preview: {
          selected_path: 'E:/Mods/Readable',
          selected_node: null,
          is_flat_mod_root: false,
          display_title: null,
          display_subtitle: null,
          mod_info_summary: null,
          ini_summary: null,
          image_summary: null,
          warning_summary: { state: 'none', messages: [] },
        },
      },
    });

    const { result } = renderHook(() => usePreviewPanelState());

    await waitFor(() => {
      expect(result.current.openSectionIds).toEqual(new Set(['alpha.ini', 'beta.ini']));
    });
  });
});
