import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createWrapper, renderHook, waitFor } from '../../../tests/testing/test-utils';
import type {
  WorkspacePreviewInput,
  WorkspacePreviewResult,
  WorkspaceStructureInput,
  WorkspaceStructureViewModel,
} from '@/shared/api/tauri/bindings.gen';
import { useWorkspacePreview, useWorkspaceStructure } from './useWorkspaceViewModel';

const state = vi.hoisted(() => ({
  explorerSubPath: 'Characters/Amber',
  selectedObjectFolderPath: 'Characters/Amber',
  selectedModPath: 'E:/Mods/Characters/Amber/A',
  currentPath: ['Characters', 'Amber'],
  selectedObjectType: null,
  objectMetaFilters: null,
  objectSortBy: null,
  objectStatusFilter: 'all' as const,
}));

const api = vi.hoisted(() => ({
  getWorkspacePreview: vi.fn(),
  getWorkspaceStructure: vi.fn(),
}));

vi.mock('@tanstack/react-query', async () => await vi.importActual('@tanstack/react-query'));

vi.mock('@/entities/game', () => ({
  useActiveGame: () => ({ activeGame: { id: 'game-1' } }),
}));

vi.mock('@/app/store', () => ({
  useAppStore: (selector: (store: typeof state) => unknown) => selector(state),
}));

vi.mock('../state/workspaceStoreBridge', () => ({
  dispatchWorkspaceRuntimeEvent: vi.fn(),
  useWorkspaceRuntimeSelector: (selector: (runtime: typeof state) => unknown) => selector(state),
}));

vi.mock('../../../shared/api/tauri/bindings', () => ({
  commands: api,
}));

function structureFixture(): WorkspaceStructureViewModel {
  return {
    objects: [],
    explorer: {
      self_node_type: null,
      self_node_kind: 'container',
      self_display_mode: 'container_folder',
      self_type_chip: null,
      self_is_mod: false,
      self_is_enabled: true,
      self_is_effectively_active: true,
      self_owner_object_id: null,
      self_owner_object_folder_path: null,
      self_classification_reasons: [],
      children: [],
      conflicts: [],
      ancestor_disabled_by: null,
      ancestor_disabled_path: null,
      inactive_reason: null,
    },
    selection: {
      selected_object_folder_path: state.selectedObjectFolderPath,
      explorer_sub_path: state.explorerSubPath,
      current_path: state.currentPath,
      reconciliation_status: 'unchanged',
      reconciliation_reason: null,
      affected_paths: [],
    },
    runtime: {
      game_id: 'game-1',
      source_state: { status: 'available', message: null },
      recovery_status: 'ready',
    },
  };
}

function previewFixture(input: WorkspacePreviewInput): WorkspacePreviewResult {
  return {
    request_identity: {
      game_id: input.game_id,
      explorer_sub_path: input.explorer_sub_path,
      selected_mod_path: input.selected_mod_path,
    },
    context_status: 'ready',
    preview: {
      selected_path: input.selected_mod_path,
      selected_node: null,
      is_flat_mod_root: false,
      display_title: null,
      display_subtitle: null,
      mod_info_summary: null,
      ini_summary: null,
      image_summary: null,
      warning_summary: { state: 'none', messages: [] },
    },
    selection: {
      selected_mod_path: input.selected_mod_path,
      reconciliation_status: 'unchanged',
      reconciliation_reason: null,
      affected_paths: [],
    },
  };
}

describe('workspace split read models', () => {
  beforeEach(() => {
    state.explorerSubPath = 'Characters/Amber';
    state.selectedObjectFolderPath = 'Characters/Amber';
    state.selectedModPath = 'E:/Mods/Characters/Amber/A';
    state.currentPath = ['Characters', 'Amber'];
    api.getWorkspaceStructure.mockReset();
    api.getWorkspacePreview.mockReset();
    api.getWorkspaceStructure.mockImplementation(async (_input: WorkspaceStructureInput) =>
      structureFixture(),
    );
    api.getWorkspacePreview.mockImplementation(async (input: WorkspacePreviewInput) =>
      previewFixture(input),
    );
  });

  it('fetches only preview data when the selected mod changes', async () => {
    const { rerender, result } = renderHook(
      () => ({ structure: useWorkspaceStructure(), preview: useWorkspacePreview() }),
      { wrapper: createWrapper },
    );

    await waitFor(() => {
      expect(result.current.structure.status).toBe('success');
      expect(result.current.preview.status).toBe('success');
      expect(api.getWorkspaceStructure).toHaveBeenCalledTimes(1);
      expect(api.getWorkspacePreview).toHaveBeenCalledTimes(1);
    });
    expect(api.getWorkspaceStructure).toHaveBeenCalledWith({
      filter: {
        game_id: 'game-1',
        object_type: null,
        search_query: null,
        meta_filters: null,
        sort_by: null,
        status_filter: null,
      },
      selected_object_folder_path: 'Characters/Amber',
      explorer_sub_path: 'Characters/Amber',
    });

    state.selectedModPath = 'E:/Mods/Characters/Amber/B';
    rerender();

    await waitFor(() => {
      expect(api.getWorkspacePreview).toHaveBeenCalledTimes(2);
    });
    expect(api.getWorkspaceStructure).toHaveBeenCalledTimes(1);
    expect(api.getWorkspacePreview).toHaveBeenLastCalledWith({
      game_id: 'game-1',
      explorer_sub_path: 'Characters/Amber',
      selected_mod_path: 'E:/Mods/Characters/Amber/B',
    });
  });
});
