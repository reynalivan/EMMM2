import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { fireEvent, render, screen, waitFor } from '../../../testing/test-utils';
import ApplyCollectionModal from './ApplyCollectionModal';
import { useAppStore } from '../../../stores/useAppStore';

vi.mock('@tanstack/react-query', async () => await vi.importActual('@tanstack/react-query'));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  convertFileSrc: (path: string) => path,
}));

function buildCurrentSnapshot(overrides: Record<string, unknown> = {}) {
  return {
    game_id: 'g-1',
    is_safe: true,
    active_collection_id: null,
    state_name: 'Unsaved Preset',
    state_kind: 'unsaved',
    roots: [],
    object_states: [
      {
        object_id: 'obj-1',
        name: 'Ainoz',
        object_type: 'Character',
        is_enabled: true,
        thumbnail_hint: null,
      },
    ],
    signature: 'current-signature',
    snapshot_source: 'disk_scan',
    reconciled_count: 0,
    ...overrides,
  };
}

function buildTargetPreview(overrides: Record<string, unknown> = {}) {
  return {
    collection: {
      id: 'c-1',
      name: 'Snapshot A',
      game_id: 'g-1',
      is_safe_context: true,
      member_count: 0,
      is_last_unsaved: false,
    },
    roots: [],
    object_states: [
      {
        object_id: 'obj-1',
        name: 'Ainoz',
        object_type: 'Character',
        is_enabled: false,
        thumbnail_hint: null,
      },
    ],
    signature: 'target-signature',
    ...overrides,
  };
}

describe('ApplyCollectionModal', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useAppStore.setState({
      activeGameId: 'g-1',
      safeMode: true,
      workspaceSelectionByCorridor: {},
    });

    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === 'get_corridor_runtime_snapshot') {
        return buildCurrentSnapshot();
      }

      if (command === 'get_collection_runtime_preview') {
        return buildTargetPreview();
      }

      if (command === 'list_collections') {
        return [buildTargetPreview().collection];
      }

      if (command === 'get_mod_thumbnail') {
        return null;
      }

      if (command === 'get_apply_progress') {
        return {
          phase: 'renaming',
          completed: 1,
          total: 2,
          current_item: 'Ainoz',
          collection_name: 'Snapshot A',
          is_safe: true,
          error: null,
        };
      }

      if (command === 'apply_collection') {
        return { changed_count: 0, warnings: [] };
      }

      throw new Error(`Unexpected command: ${String(command)}`);
    });
  });

  it('shows object-state-only groups when no mods are present', async () => {
    render(<ApplyCollectionModal collectionId="c-1" collectionName="Snapshot A" onClose={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getAllByText('Ainoz').length).toBeGreaterThan(0);
    });

    expect(screen.getByText('Disabled')).toBeInTheDocument();
    expect(screen.getAllByText('No mods in this object.').length).toBeGreaterThan(0);
  });

  it('disables apply confirmation when the target is already the active named state', async () => {
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === 'get_corridor_runtime_snapshot') {
        return buildCurrentSnapshot({
          active_collection_id: 'c-1',
          state_name: 'Snapshot A',
          state_kind: 'named',
          object_states: [],
          signature: 'sig-active',
        });
      }

      if (command === 'get_collection_runtime_preview') {
        return buildTargetPreview({
          object_states: [],
          signature: 'sig-active',
        });
      }

      if (command === 'list_collections') {
        return [buildTargetPreview().collection];
      }

      if (command === 'get_apply_progress') {
        return {
          phase: 'idle',
          completed: 0,
          total: 0,
          current_item: null,
          collection_name: 'Snapshot A',
          is_safe: true,
          error: null,
        };
      }

      if (command === 'get_mod_thumbnail') {
        return null;
      }

      if (command === 'apply_collection') {
        return { changed_count: 0, warnings: [] };
      }

      throw new Error(`Unexpected command: ${String(command)}`);
    });

    render(<ApplyCollectionModal collectionId="c-1" collectionName="Snapshot A" onClose={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByText('This collection is already the active named state.')).toBeInTheDocument();
    });

    expect(screen.getByTestId('modal-apply-btn')).toBeDisabled();
  });

  it('stores the applied collection as the selected row for the current corridor', async () => {
    const onClose = vi.fn();

    render(<ApplyCollectionModal collectionId="c-1" collectionName="Snapshot A" onClose={onClose} />);

    await waitFor(() => {
      expect(screen.getByTestId('modal-apply-btn')).not.toBeDisabled();
    });

    fireEvent.click(screen.getByTestId('modal-apply-btn'));

    await waitFor(() => {
      expect(useAppStore.getState().workspaceSelectionByCorridor).toEqual({
        'g-1::safe': {
          kind: 'stored_collection',
          collection_id: 'c-1',
        },
      });
    });

    expect(screen.getByText('Apply complete')).toBeInTheDocument();
    expect(onClose).not.toHaveBeenCalled();
  });
});
