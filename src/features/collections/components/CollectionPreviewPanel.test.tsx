import { render, screen } from '../../../tests/testing/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { CollectionPreview } from '../../../types/collection';
import { CollectionPreviewPanel } from './CollectionPreviewPanel';

const mockUseCollectionPreview = vi.fn();

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string | { count: number }) =>
      typeof fallback === 'string' ? fallback : key,
  }),
}));

vi.mock('../hooks/useCollections', () => ({
  useCollectionPreview: (...args: unknown[]) => mockUseCollectionPreview(...args),
}));

vi.mock('./CollectionTreeView', () => ({
  CollectionTreeView: () => null,
}));

vi.mock('../../../core/lib/runtimeLabels', () => ({
  useRuntimeLabels: () => ({ currentChanges: 'Current changes' }),
  getCollectionDisplayName: ({ name, isUnsaved }: { name: string | null; isUnsaved: boolean }) =>
    isUnsaved || !name ? 'Current changes' : name,
}));

function createPreview(): CollectionPreview {
  return {
    collection: {
      id: 'collection-1',
      name: 'Named collection',
      is_safe: true,
      is_safety_classified: true,
      is_active: false,
      signature: null,
      updated_at: '2026-08-28T00:00:00Z',
      mod_count: 1,
    },
    tree_nodes: [],
    projected_state: {
      object_states: [],
      active_roots: [],
      summary: {
        object_count: 0,
        enabled_object_count: 0,
        active_root_count: 1,
        missing_root_count: 0,
      },
    },
  };
}

describe('CollectionPreviewPanel', () => {
  beforeEach(() => {
    mockUseCollectionPreview.mockReturnValue({
      isLoading: false,
      data: createPreview(),
    });
  });

  it('uses the stored collection name', () => {
    render(
      <CollectionPreviewPanel
        source={{ kind: 'stored_collection', collectionId: 'collection-1' }}
        gameId="game-1"
        runtimeSnapshot={undefined}
      />,
    );

    expect(screen.getByText('Named collection')).toBeInTheDocument();
    expect(screen.queryByText('Current changes')).not.toBeInTheDocument();
  });
});
