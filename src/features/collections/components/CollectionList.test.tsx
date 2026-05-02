import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '../../../testing/test-utils';
import { CollectionList } from './CollectionList';
import type { CollectionSummary } from '../../../types/collection';
import type { CollectionListRow } from '../types';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallbackOrOptions?: string | { count?: number; mode?: string }) => {
      const labels: Record<string, string> = {
        'layout:context.unsaved_safe': 'Unsaved SAFE Preset',
        'layout:context.unsaved_unsafe': 'Unsaved UNSAFE Preset',
      };

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
    is_unsaved: true,
    is_active: true,
    is_undo_target: false,
    signature: null,
    updated_at: '2026-03-25T15:30:00Z',
    member_count: 4,
    mod_count: 4,
    ...partial,
  };
}

describe('CollectionList', () => {
  it('shows Save instead of Apply for active unsaved collections', () => {
    const onSave = vi.fn();
    const rows: CollectionListRow[] = [
      {
        kind: 'stored_collection',
        rowId: 'unsaved-1',
        collection: createCollection({}),
      },
    ];

    render(
      <CollectionList
        rows={rows}
        selectedId="unsaved-1"
        isLoading={false}
        safeMode={true}
        onSelect={vi.fn()}
        onApply={vi.fn()}
        onDelete={vi.fn()}
        onRename={vi.fn()}
        onSave={onSave}
        isApplying={false}
        isDeleting={false}
      />,
    );

    expect(screen.getByText('Unsaved SAFE Preset')).toBeInTheDocument();
    const saveButton = screen.getByRole('button', { name: /Save/i });
    expect(saveButton).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /Apply/i })).not.toBeInTheDocument();

    fireEvent.click(saveButton);
    expect(onSave).toHaveBeenCalledWith({
      mode: 'save_current_state',
      sourceCollectionId: null,
    });
  });

  it('renders the synthetic current runtime row as a live save-current source', () => {
    const rows: CollectionListRow[] = [
      {
        kind: 'current_runtime',
        rowId: '__current_runtime__',
        label: 'Unsaved SAFE Preset',
        isSafe: true,
        modCount: 2,
        isActive: true,
      },
    ];

    render(
      <CollectionList
        rows={rows}
        selectedId="__current_runtime__"
        isLoading={false}
        safeMode={true}
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
