import { describe, expect, it, vi } from 'vitest';
import { render, screen } from '../../../tests/testing/test-utils';
import { CollectionList } from './CollectionList';
import { buildCollectionWorkspaceRows, filterCollectionRowsBySafety } from '../types';
import type { CollectionRuntimeSnapshot, CollectionSummary } from '../../../types/collection';
import type { CollectionListRow } from '../types';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallbackOrOptions?: string | { count?: number; mode?: string }) => {
      const labels: Record<string, string> = {};

      if (key in labels) {
        return labels[key];
      }

      if (typeof fallbackOrOptions === 'string') {
        return fallbackOrOptions;
      }

      if (key === 'list.item.mod_count') {
        return `${fallbackOrOptions?.count ?? 0} mods`;
      }

      return key;
    },
  }),
}));

function createCollection(partial: Partial<CollectionSummary>): CollectionSummary {
  return {
    id: 'unsaved-1',
    name: '202603251530',
    is_safe: true,
    is_safety_classified: true,
    is_active: true,
    signature: null,
    updated_at: '2026-03-25T15:30:00Z',
    mod_count: 4,
    ...partial,
  };
}

function createRuntimeSnapshot(
  partial: Partial<CollectionRuntimeSnapshot> = {},
): CollectionRuntimeSnapshot {
  return {
    game_id: 'game-1',
    active_collection_id: null,
    active_collection_name: null,
    current_signature: 'signature-1',
    is_dirty: false,
    runtime_status: 'unsaved',
    is_safe: true,
    is_safety_classified: true,
    missing_count: 0,
    last_changes: null,
    current_mods: [],
    current_objects: [],
    current_tree_nodes: [],
    projected_state: {
      object_states: [],
      active_roots: [],
      summary: {
        object_count: 0,
        enabled_object_count: 0,
        active_root_count: 0,
        missing_root_count: 0,
      },
    },
    ...partial,
  };
}

describe('CollectionList', () => {
  it('uses a virtual current-runtime row instead of a stored unsaved collection', () => {
    const storedCollection = createCollection({ id: 'stored-1', name: 'Stored collection' });
    const runtime = createRuntimeSnapshot({
      is_dirty: true,
      current_mods: [
        {
          kind: 'mod',
          collection_id: 'runtime',
          mod_id: 'mod-1',
          mod_path: 'Mods/Mod 1',
          mod_path_key: 'mods/mod 1',
          object_id: 'root',
          is_enabled: true,
          display_name: 'Mod 1',
          preview_path: null,
          node_type: null,
          warnings: [],
          is_safe: true,
          safety_source: null,
        },
      ],
    });

    expect(buildCollectionWorkspaceRows([storedCollection], runtime, 'Current changes')).toEqual([
      {
        kind: 'current_runtime',
        rowId: '__current_runtime__',
        label: 'Current changes',
        modCount: 0,
        isActive: true,
        isSafe: true,
        isSafetyClassified: true,
      },
      {
        kind: 'stored_collection',
        rowId: storedCollection.id,
        collection: storedCollection,
      },
    ]);
  });

  it('does not create a virtual row for an empty unsaved workspace', () => {
    const runtime = createRuntimeSnapshot({ is_dirty: true });

    expect(buildCollectionWorkspaceRows([], runtime, 'Current changes')).toEqual([]);
  });

  it('filters stored collections and current changes by safety', () => {
    const safe = createCollection({ id: 'safe', is_safe: true });
    const unsafe = createCollection({ id: 'unsafe', is_safe: false });
    const rows = [
      {
        kind: 'current_runtime' as const,
        rowId: '__current_runtime__',
        label: 'Current changes',
        modCount: 1,
        isActive: true,
        isSafe: true,
        isSafetyClassified: true,
      },
      { kind: 'stored_collection' as const, rowId: safe.id, collection: safe },
      { kind: 'stored_collection' as const, rowId: unsafe.id, collection: unsafe },
    ];

    expect(filterCollectionRowsBySafety(rows, 'safe').map((row) => row.rowId)).toEqual([
      '__current_runtime__',
      'safe',
    ]);
    expect(filterCollectionRowsBySafety(rows, 'unsafe').map((row) => row.rowId)).toEqual([
      'unsafe',
    ]);
  });

  it('hides unclassified collections from both classified filters', () => {
    const unknown = createCollection({
      id: 'unknown',
      is_safe: true,
      is_safety_classified: false,
    });
    const rows: CollectionListRow[] = [
      { kind: 'stored_collection', rowId: unknown.id, collection: unknown },
    ];

    expect(filterCollectionRowsBySafety(rows, 'all')).toEqual(rows);
    expect(filterCollectionRowsBySafety(rows, 'safe')).toEqual([]);
    expect(filterCollectionRowsBySafety(rows, 'unsafe')).toEqual([]);
  });

  it('shows a mixed unsafe and unknown collection under Unsafe only', () => {
    const mixed = createCollection({
      id: 'mixed',
      is_safe: false,
      is_safety_classified: false,
    });
    const rows: CollectionListRow[] = [
      { kind: 'stored_collection', rowId: mixed.id, collection: mixed },
    ];

    expect(filterCollectionRowsBySafety(rows, 'safe')).toEqual([]);
    expect(filterCollectionRowsBySafety(rows, 'unsafe')).toEqual(rows);
  });
  it('treats a stored collection as named even when a legacy unsaved flag is present', () => {
    const rows: CollectionListRow[] = [
      {
        kind: 'stored_collection',
        rowId: 'unsaved-1',
        collection: createCollection({ is_active: false }),
      },
    ];

    render(
      <CollectionList
        rows={rows}
        selectedId="unsaved-1"
        isLoading={false}
        onSelect={vi.fn()}
        onApply={vi.fn()}
        onDelete={vi.fn()}
        onRename={vi.fn()}
        onSave={vi.fn()}
        isApplying={false}
        isDeleting={false}
      />,
    );

    expect(screen.getByText('202603251530')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Apply/i })).toBeInTheDocument();
    expect(screen.getByTitle('list.item.rename')).toBeInTheDocument();
    expect(screen.getByTitle('list.item.delete')).toBeInTheDocument();
  });

  it('renders the synthetic current runtime row as a live save-current source', () => {
    const rows: CollectionListRow[] = [
      {
        kind: 'current_runtime',
        rowId: '__current_runtime__',
        label: 'Current changes',
        modCount: 2,
        isActive: true,
        isSafe: true,
        isSafetyClassified: true,
      },
    ];

    render(
      <CollectionList
        rows={rows}
        selectedId="__current_runtime__"
        isLoading={false}
        onSelect={vi.fn()}
        onApply={vi.fn()}
        onDelete={vi.fn()}
        onRename={vi.fn()}
        onSave={vi.fn()}
        isApplying={false}
        isDeleting={false}
      />,
    );

    expect(screen.getByText('Live')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Save/i })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /Apply/i })).not.toBeInTheDocument();
  });
});
