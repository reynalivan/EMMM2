import {
  previewDataMocks,
  workspaceQueryMock,
  selectionOverrideState,
  createMockQuery,
  createMockMutation,
  setupDefaultMocks,
} from './previewPanelState.test-fixtures';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { act, cleanup, renderHook, waitFor } from '../../../tests/testing/test-utils';
import { usePreviewPanelState } from './usePreviewPanelState';
import { useAppStore } from '@/app/store';
const selectionOverride = selectionOverrideState();

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

  it('preserves container metadata and edit mode through a query gap and persists at the same identity destination', async () => {
    vi.useFakeTimers();
    const oldPath = 'E:/Mods/Container';
    const destination = 'E:/Mods/DISABLED Container';
    const metadata = {
      actual_name: 'Container',
      author: 'Author',
      version: '1.0',
      description: 'Original',
    };
    const preview = (path: string, filesystemIdentity = 'container-id') => ({
      selected_path: path,
      selected_node: {
        path,
        id: null,
        node_kind: 'container',
        filesystem_identity: filesystemIdentity,
        display_name: 'Container',
      },
      display_title: 'Container',
      mod_info_summary: metadata,
    });
    const setPreview = (path: string, pending = false, filesystemIdentity = 'container-id') => {
      const query = createMockQuery({ preview: preview(path, filesystemIdentity) });
      query.isPending = pending;
      workspaceQueryMock().mockReturnValue(query);
      selectionOverride.path = path;
    };
    const mutation = createMockMutation();
    mutation.mutateAsync.mockResolvedValue(undefined);
    previewDataMocks().useUpdateModInfoDetails.mockReturnValue(mutation);
    setPreview(oldPath);
    const { result, rerender } = renderHook(() => usePreviewPanelState());
    act(() => {
      result.current.setDescriptionDraft('QA pending toggle');
      result.current.setMetadataEditing(true);
    });
    setPreview(destination, true);
    rerender();
    expect(result.current.activePath).toBeNull();
    expect(result.current.descriptionDraft).toBe('QA pending toggle');
    expect(result.current.metadataDirty).toBe(true);
    expect(result.current.isMetadataEditing).toBe(true);
    await act(async () => vi.advanceTimersByTimeAsync(5000));
    expect(mutation.mutateAsync).not.toHaveBeenCalled();
    setPreview(destination);
    rerender();
    expect(result.current.isMetadataEditing).toBe(true);
    expect(result.current.descriptionDraft).toBe('QA pending toggle');
    await act(async () => vi.advanceTimersByTimeAsync(2500));
    expect(mutation.mutateAsync).toHaveBeenCalledWith({
      folderPath: destination,
      update: { ...metadata, description: 'QA pending toggle' },
    });
    expect(result.current.metadataDirty).toBe(false);
    setPreview(destination, false, 'another-container-id');
    rerender();
    expect(result.current.isMetadataEditing).toBe(false);
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

  it('syncs activePath from selection without requiring timer flush', async () => {
    vi.useFakeTimers();

    const selectedPath = 'E:/Mods/Parent/VariantA';
    const useWorkspaceViewModelMock = workspaceQueryMock();
    useWorkspaceViewModelMock.mockReturnValue({
      data: {
        preview: {
          selected_path: selectedPath,
          selected_node: {
            node_type: 'ContainerFolder',
            classification_reasons: [],
            name: 'VariantA',
            folder_name: 'VariantA',
            path: selectedPath,
            is_enabled: true,
            is_directory: true,
            thumbnail_path: null,
            modified_at: 0,
            size_bytes: 0,
            has_info_json: false,
            is_favorite: false,
            is_misplaced: false,
            is_safe: true,
            metadata: null,
            category: null,
            conflict_group_id: null,
            conflict_state: null,
            warnings: [],
            node_kind: 'container',
            display_mode: 'container_folder',
            type_chip: null,
            display_name: 'VariantA',
            is_effectively_active: true,
            ancestor_disabled: false,
            inactive_reason: null,
            warning_state: 'none',
            primary_warning: null,
            can_navigate: true,
          },
          is_flat_mod_root: false,
          display_title: 'VariantA',
          display_subtitle: null,
          mod_info_summary: {
            actual_name: 'VariantA',
            author: 'Unknown',
            version: '1.0',
            description: '',
            is_safe: true,
            is_favorite: false,
            has_info_json: false,
          },
          ini_summary: { file_count: 0, file_names: [] },
          image_summary: { image_count: 0, primary_image_path: null },
          warning_summary: { state: 'none', messages: [] },
        },
      },
    });

    const { result } = renderHook(() => usePreviewPanelState());

    await act(async () => {
      await Promise.resolve();
    });

    expect(result.current.activePath).toBe(selectedPath);
    vi.useRealTimers();
  });

  it('keeps a selected conflict path but suppresses all preview detail queries', () => {
    const selectedPath = 'E:/Mods/Alice/Blue';
    const group = {
      group_id: 'alice-blue',
      identity: 'alice/blue',
      display_name: 'Blue',
      candidates: [
        {
          path: selectedPath,
          folder_name: 'Blue',
          base_name: 'Blue',
          is_enabled: true,
        },
        {
          path: 'E:/Mods/Alice/DISABLED Blue',
          folder_name: 'DISABLED Blue',
          base_name: 'Blue',
          is_enabled: false,
        },
      ],
    };
    const appState = useAppStore.getState() as unknown as {
      folderConflictsByGame: Record<string, unknown[]>;
    };
    appState.folderConflictsByGame = { GIMI: [group] };
    workspaceQueryMock().mockReturnValue({
      data: {
        preview: {
          selected_path: selectedPath,
          selected_node: null,
          display_title: 'Blue',
          display_subtitle: null,
          mod_info_summary: null,
          warning_summary: { state: 'none', messages: [] },
        },
      },
    });

    const { result } = renderHook(() => usePreviewPanelState());

    expect(result.current.activePath).toBe(selectedPath);
    expect(result.current.folderNameConflict).toEqual(group);
    expect(previewDataMocks().useModIniDocuments).toHaveBeenCalledWith(null);
    expect(previewDataMocks().usePreviewImages).toHaveBeenCalledWith(null);
  });

  // Covers: TC-6.1-01 (Title and description sync from workspace preview summary)
  it('should sync title and description from workspace preview summary', async () => {
    const useWorkspaceViewModelMock = workspaceQueryMock();
    useWorkspaceViewModelMock.mockReturnValue({
      data: {
        preview: {
          selected_path: 'E:/Mods/Test',
          selected_node: null,
          is_flat_mod_root: false,
          display_title: 'Test Mod',
          display_subtitle: 'Author • v1.0',
          mod_info_summary: {
            actual_name: 'Test Mod',
            author: 'Author',
            version: '1.0',
            description: 'A test mod',
            is_safe: true,
            is_favorite: false,
            has_info_json: true,
          },
          ini_summary: { file_count: 0, file_names: [] },
          image_summary: { image_count: 0, primary_image_path: null },
          warning_summary: { state: 'none', messages: [] },
        },
      },
    });

    const { result } = renderHook(() => usePreviewPanelState());

    await waitFor(() => {
      expect(result.current.titleDraft).toBe('Test Mod');
      expect(result.current.descriptionDraft).toBe('A test mod');
    });
  });

  // Covers: TC-6.2-01 (Gallery image list from usePreviewImages)
});
