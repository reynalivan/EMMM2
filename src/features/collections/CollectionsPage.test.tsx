import { render, screen, waitFor, fireEvent, within } from '../../testing/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import CollectionsPage from './CollectionsPage';
import { useAppStore } from '../../stores/useAppStore';

vi.mock('@tanstack/react-query', async () => await vi.importActual('@tanstack/react-query'));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  convertFileSrc: (path: string) => path,
}));

vi.mock('../../lib/services/scanService', () => ({
  scanService: {
    syncDatabase: vi.fn().mockResolvedValue({}),
  },
}));

vi.mock('../../hooks/useActiveGame', () => ({
  useActiveGame: () => ({
    activeGame: {
      id: 'g-1',
      name: 'Genshin',
      game_type: 'GIMI',
      mod_path: 'E:/Mods',
      game_exe: 'E:/Game/Genshin.exe',
      loader_exe: null,
      launch_args: null,
    },
  }),
}));

function buildCollections() {
  return [
    {
      id: 'c-unsaved',
      name: 'Unsaved 202603182218',
      game_id: 'g-1',
      is_safe_context: true,
      member_count: 4,
      is_last_unsaved: true,
    },
    {
      id: 'c-1',
      name: 'Abyss Team',
      game_id: 'g-1',
      is_safe_context: true,
      member_count: 5,
      is_last_unsaved: false,
    },
  ];
}

function buildRuntimeSnapshot(overrides: Record<string, unknown> = {}) {
  return {
    game_id: 'g-1',
    is_safe: true,
    active_collection_id: null,
    state_name: 'Unsaved Preset',
    state_kind: 'unsaved',
    roots: [
      {
        id: 'runtime-mod',
        object_id: 'obj-1',
        object_name: 'Hu Tao',
        object_type: 'Character',
        actual_name: 'Live Runtime Mod',
        folder_path: 'E:/Mods/Hu Tao/Live Runtime Mod',
        is_safe: true,
        node_type: 'FlatModRoot',
      },
    ],
    object_states: [
      {
        object_id: 'obj-1',
        is_enabled: true,
        name: 'Hu Tao',
        object_type: 'Character',
        thumbnail_hint: null,
      },
    ],
    signature: 'runtime-signature',
    snapshot_source: 'disk_scan',
    reconciled_count: 0,
    ...overrides,
  };
}

function buildCollectionPreview(collectionId: string) {
  if (collectionId === 'c-unsaved') {
    return {
      collection: buildCollections()[0],
      roots: [
        {
          id: 'snapshot-mod',
          object_id: 'obj-1',
          object_name: 'Hu Tao',
          object_type: 'Character',
          actual_name: 'Stored Snapshot Mod',
          folder_path: 'E:/Mods/Hu Tao/Stored Snapshot Mod',
          is_safe: true,
          node_type: 'FlatModRoot',
        },
      ],
      object_states: [
        {
          object_id: 'obj-1',
          is_enabled: true,
          name: 'Hu Tao',
          object_type: 'Character',
          thumbnail_hint: null,
        },
      ],
      signature: 'unsaved-signature',
    };
  }

  if (collectionId === 'c-snapshot') {
    return {
      collection: {
        id: 'c-snapshot',
        name: 'Saved Snapshot',
        game_id: 'g-1',
        is_safe_context: true,
        member_count: 4,
        is_last_unsaved: false,
      },
      roots: [
        {
          id: 'snapshot-mod',
          object_id: 'obj-1',
          object_name: 'Hu Tao',
          object_type: 'Character',
          actual_name: 'Stored Snapshot Mod',
          folder_path: 'E:/Mods/Hu Tao/Stored Snapshot Mod',
          is_safe: true,
          node_type: 'FlatModRoot',
        },
      ],
      object_states: [
        {
          object_id: 'obj-1',
          is_enabled: true,
          name: 'Hu Tao',
          object_type: 'Character',
          thumbnail_hint: null,
        },
      ],
      signature: 'saved-snapshot-signature',
    };
  }

  return {
    collection: buildCollections()[1],
    roots: [
      {
        id: 'named-mod',
        object_id: 'obj-1',
        object_name: 'Hu Tao',
        object_type: 'Character',
        actual_name: 'Named Collection Mod',
        folder_path: 'E:/Mods/Hu Tao/Named Collection Mod',
        is_safe: true,
        node_type: 'FlatModRoot',
      },
    ],
    object_states: [
      {
        object_id: 'obj-1',
        is_enabled: true,
        name: 'Hu Tao',
        object_type: 'Character',
        thumbnail_hint: null,
      },
    ],
    signature: 'named-signature',
  };
}

function mockCollectionsPageRuntime(runtimeOverrides: Record<string, unknown> = {}) {
  const collectionsState = [...buildCollections()];

  vi.mocked(invoke).mockImplementation(async (command, args) => {
    if (command === 'list_collections') {
      return collectionsState;
    }

    if (command === 'get_corridor_runtime_snapshot') {
      return buildRuntimeSnapshot(runtimeOverrides);
    }

    if (command === 'get_collection_runtime_preview') {
      const collectionId = (args as { collectionId?: string } | undefined)?.collectionId;
      return buildCollectionPreview(collectionId ?? 'c-1');
    }

    if (command === 'get_mod_thumbnail') {
      return null;
    }

    if (command === 'save_snapshot_collection_as_named') {
      const savedCollection = {
        id: 'c-snapshot',
        name: 'Saved Snapshot',
        game_id: 'g-1',
        is_safe_context: true,
        member_count: 4,
        is_last_unsaved: false,
      };
      collectionsState.push(savedCollection);
      return {
        collection: savedCollection,
        mod_ids: ['snapshot-mod'],
        object_states: [{ object_id: 'obj-1', is_enabled: true }],
      };
    }

    if (command === 'create_collection') {
      const createdCollection = {
        id: 'c-new',
        name: 'New Current Save',
        game_id: 'g-1',
        is_safe_context: true,
        member_count: 1,
        is_last_unsaved: false,
      };
      collectionsState.push(createdCollection);
      return {
        collection: createdCollection,
        mod_ids: ['runtime-mod'],
        object_states: [{ object_id: 'obj-1', is_enabled: true }],
      };
    }

    if (command === 'update_collection') {
      return {
        collection: buildCollections()[1],
        mod_ids: ['named-mod'],
        object_states: [{ object_id: 'obj-1', is_enabled: true }],
      };
    }

    if (command === 'apply_collection') {
      return { changed_count: 1, warnings: [] };
    }

    if (command === 'get_apply_progress') {
      return {
        phase: 'idle',
        completed: 0,
        total: 0,
        current_item: null,
        collection_name: 'Abyss Team',
        is_safe: true,
        error: null,
      };
    }

    throw new Error(`Unexpected command: ${String(command)} ${JSON.stringify(args)}`);
  });
}

describe('CollectionsPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useAppStore.setState({
      safeMode: true,
      gridSelection: new Set(['mod-a']),
      activeGameId: 'g-1',
      workspaceSelectionByCorridor: {},
    });
  });

  it('shows current runtime and stored unsaved rows separately when strict state is unsaved', async () => {
    mockCollectionsPageRuntime();

    render(<CollectionsPage />);

    await waitFor(() => {
      expect(screen.getAllByText('Unsaved Preset').length).toBeGreaterThan(0);
    });

    expect(screen.getByText('Unsaved 202603182218')).toBeInTheDocument();
    expect(screen.getByText('Live Runtime Mod')).toBeInTheDocument();

    fireEvent.click(screen.getByText('Unsaved 202603182218'));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('get_collection_runtime_preview', {
        collectionId: 'c-unsaved',
        gameId: 'g-1',
      });
    });

    await waitFor(() => {
      expect(screen.getByText('Stored Snapshot Mod')).toBeInTheDocument();
    });
  });

  it('keeps stored unsaved snapshot visible when strict state is named and selects the active named collection', async () => {
    mockCollectionsPageRuntime({
      active_collection_id: 'c-1',
      state_name: 'Abyss Team',
      state_kind: 'named',
      roots: [],
    });

    render(<CollectionsPage />);

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('get_collection_runtime_preview', {
        collectionId: 'c-1',
        gameId: 'g-1',
      });
    });

    expect(screen.getByText('Unsaved 202603182218')).toBeInTheDocument();
    await waitFor(() => {
      expect(screen.getByText('Named Collection Mod')).toBeInTheDocument();
    });
    expect(screen.queryByText('Live Runtime Mod')).toBeNull();
  });

  it('saves the stored unsaved snapshot through save_snapshot_collection_as_named', async () => {
    mockCollectionsPageRuntime({
      active_collection_id: 'c-1',
      state_name: 'Abyss Team',
      state_kind: 'named',
      roots: [],
    });

    render(<CollectionsPage />);

    await waitFor(() => {
      expect(screen.getByText('Unsaved 202603182218')).toBeInTheDocument();
    });

    const unsavedRow = screen.getByText('Unsaved 202603182218').closest('tr');
    expect(unsavedRow).not.toBeNull();
    fireEvent.click(within(unsavedRow as HTMLElement).getByRole('button', { name: /save as/i }));

    const input = await screen.findByPlaceholderText('e.g. Abyss Run 1');
    fireEvent.change(input, { target: { value: 'Saved Snapshot' } });
    fireEvent.click(screen.getByRole('button', { name: /save snapshot as collection/i }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('save_snapshot_collection_as_named', {
        sourceCollectionId: 'c-unsaved',
        gameId: 'g-1',
        name: 'Saved Snapshot',
      });
    });

    await waitFor(() => {
      expect(useAppStore.getState().workspaceSelectionByCorridor).toEqual({
        'g-1::safe': {
          kind: 'stored_collection',
          collection_id: 'c-snapshot',
        },
      });
    });
  });

  it('restores persisted workspace selection before strict active fallback', async () => {
    mockCollectionsPageRuntime({
      active_collection_id: 'c-1',
      state_name: 'Abyss Team',
      state_kind: 'named',
      roots: [],
    });

    useAppStore.setState({
      workspaceSelectionByCorridor: {
        'g-1::safe': {
          kind: 'stored_collection',
          collection_id: 'c-unsaved',
        },
      },
    });

    render(<CollectionsPage />);

    await waitFor(() => {
      expect(screen.getByText('Stored Snapshot Mod')).toBeInTheDocument();
    });

    expect(screen.getAllByText('Unsaved 202603182218').length).toBeGreaterThan(0);
    expect(screen.queryByText('Named Collection Mod')).toBeNull();
  });
});
