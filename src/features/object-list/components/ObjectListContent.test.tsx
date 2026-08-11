import { act, render, renderHook, screen } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import ObjectListContent from './ObjectListContent';
import { buildObjectContextMenuTarget } from './ObjectContextMenuTarget';
import type { FlatItem } from '../hooks/useObjectListVirtualizer';
import type { WorkspaceCapabilities, WorkspaceObjectNode } from '../../../types/workspace';
import { useObjectBulkSelect } from '../hooks/useObjectBulkSelect';

vi.mock('../../../components/ui/ContextMenu', () => ({
  ContextMenu: ({ children, content }: { children: React.ReactNode; content: React.ReactNode }) => (
    <div>
      <div data-testid="context-content">{content}</div>
      {children}
    </div>
  ),
}));
vi.mock('./ObjectRowItem', () => ({
  default: ({
    obj,
    onToggleBulkSelect,
  }: {
    obj: { name: string };
    onToggleBulkSelect?: () => void;
  }) => (
    <div data-testid="row-item" data-bulk-selectable={Boolean(onToggleBulkSelect)}>
      {obj.name}
    </div>
  ),
}));
vi.mock('./CategorySection', () => ({
  default: ({ category }: { category: { name: string } }) => (
    <div data-testid="category-section">{category.name}</div>
  ),
}));
vi.mock('./ObjectContextMenu', () => ({
  ObjectContextMenu: ({ item }: { item: unknown }) => (
    <div data-testid="object-context-target">{JSON.stringify(item)}</div>
  ),
}));

describe('ObjectListContent', () => {
  const baseCapabilities: WorkspaceCapabilities = {
    can_toggle: true,
    can_rename: true,
    can_delete: true,
    can_move: false,
    can_toggle_safe: false,
    can_sync: true,
    can_enable_only_this: false,
    can_pin: true,
    can_edit_metadata: true,
    can_reveal_in_explorer: true,
    can_move_category: true,
    can_open_in_explorer: true,
  };

  const objectRow: WorkspaceObjectNode = {
    id: '1',
    name: 'Obj1',
    display_name: 'Obj1',
    is_registered: true,
    node_kind: 'object',
    display_mode: 'unknown',
    type_chip: null,
    folder_path: 'Characters/Obj1',
    matched_entry_key: null,
    matched_alias_name: null,
    matched_confidence: null,
    matched_reason: null,
    matched_source: null,
    object_type: 'Character',
    sub_category: null,
    status: 1,
    created_at: null,
    mod_count: 3,
    enabled_count: 2,
    thumbnail_path: null,
    is_pinned: true,
    is_auto_sync: false,
    is_object_disabled: false,
    has_naming_conflict: false,
    is_effectively_active: true,
    inactive_reason: null,
    warning_state: 'none',
    primary_warning: null,
    switch_state: 'enabled',
    switch_reason: null,
    switch_policy_key: 'object',
    capabilities: baseCapabilities,
    metadata: '{}',
    tags: '[]',
    hash_db: null,
    custom_skins: null,
    active_mod_paths: null,
  };

  it('renders virtualized list correctly', () => {
    const mockVirtualizerFactory = () => {
      const totalSize = 100;
      const virtualItems = [
        { index: 0, size: 50, start: 0 },
        { index: 1, size: 50, start: 50 },
      ];
      return {
        getTotalSize: () => totalSize,
        getVirtualItems: () => virtualItems,
      };
    };

    const flatItems: FlatItem[] = [
      {
        type: 'header',
        category: { name: 'Chars' } as unknown as React.ComponentProps<
          typeof ObjectListContent
        >['flatObjectItems'][0] extends { type: 'header' }
          ? React.ComponentProps<typeof ObjectListContent>['flatObjectItems'][0]['category']
          : never,
        count: 1,
      },
      {
        type: 'row',
        obj: objectRow,
      },
    ];

    render(
      <ObjectListContent
        parentRef={{ current: null }}
        rowVirtualizer={
          mockVirtualizerFactory() as unknown as import('@tanstack/react-virtual').Virtualizer<
            HTMLDivElement,
            Element
          >
        }
        flatObjectItems={flatItems}
        selectedObjectFolderPath={null}
        selectedObjectType={null}
        onSelectObject={vi.fn()}
        setSelectedObjectType={vi.fn()}
        isMobile={false}
        stickyPosition={null}
        selectedIndex={-1}
        scrollToSelected={vi.fn()}
        contextMenuProps={
          {} as unknown as React.ComponentProps<typeof ObjectListContent>['contextMenuProps']
        }
      />,
    );

    expect(screen.getByTestId('category-section')).toBeInTheDocument();
    expect(screen.getByTestId('row-item')).toBeInTheDocument();
  });

  it('keeps filesystem-only roots out of bulk selection', () => {
    const runtimeRoot: WorkspaceObjectNode = {
      ...objectRow,
      id: 'fs-root',
      is_registered: false,
    };
    const flatItems: FlatItem[] = [
      { type: 'row', obj: objectRow },
      { type: 'row', obj: runtimeRoot },
    ];
    const { result } = renderHook(() => useObjectBulkSelect(flatItems));

    act(() => result.current.selectAll());

    expect([...result.current.selectedIds]).toEqual(['1']);
  });

  it('does not expose a bulk checkbox for filesystem-only roots', () => {
    const runtimeRoot: WorkspaceObjectNode = {
      ...objectRow,
      id: 'fs-root',
      is_registered: false,
    };
    const virtualizer = {
      getTotalSize: () => 70,
      getVirtualItems: () => [{ index: 0, size: 70, start: 0 }],
    };

    render(
      <ObjectListContent
        parentRef={{ current: null }}
        rowVirtualizer={
          virtualizer as unknown as import('@tanstack/react-virtual').Virtualizer<
            HTMLDivElement,
            Element
          >
        }
        flatObjectItems={[{ type: 'row', obj: runtimeRoot }]}
        selectedObjectFolderPath={null}
        selectedObjectType={null}
        onSelectObject={vi.fn()}
        setSelectedObjectType={vi.fn()}
        isMobile={false}
        stickyPosition={null}
        selectedIndex={-1}
        scrollToSelected={vi.fn()}
        onToggleBulkSelect={vi.fn()}
        contextMenuProps={
          {} as unknown as React.ComponentProps<typeof ObjectListContent>['contextMenuProps']
        }
      />,
    );

    expect(screen.getByTestId('row-item')).toHaveAttribute('data-bulk-selectable', 'false');
  });

  it('uses identical object context menu targets for row and sticky row', () => {
    const mockVirtualizerFactory = () => ({
      getTotalSize: () => 70,
      getVirtualItems: () => [{ index: 0, size: 70, start: 0 }],
    });

    const flatItems: FlatItem[] = [{ type: 'row', obj: objectRow }];

    render(
      <ObjectListContent
        parentRef={{ current: null }}
        rowVirtualizer={
          mockVirtualizerFactory() as unknown as import('@tanstack/react-virtual').Virtualizer<
            HTMLDivElement,
            Element
          >
        }
        flatObjectItems={flatItems}
        selectedObjectFolderPath={objectRow.folder_path}
        selectedObjectType={null}
        onSelectObject={vi.fn()}
        setSelectedObjectType={vi.fn()}
        isMobile={false}
        stickyPosition="bottom"
        selectedIndex={0}
        scrollToSelected={vi.fn()}
        contextMenuProps={{
          isSyncing: false,
          categoryNames: [{ name: 'Character', label: 'Characters' }],
          handleEdit: vi.fn(),
          handleSyncWithDb: vi.fn(),
          handleDeleteObject: vi.fn(),
          handlePin: vi.fn(),
          handleMoveCategory: vi.fn(),
          handleRevealInExplorer: vi.fn(),
          handleEnableObject: vi.fn(),
          handleDisableObject: vi.fn(),
        }}
      />,
    );

    const targets = screen
      .getAllByTestId('object-context-target')
      .map((node) => JSON.parse(node.textContent ?? '{}'));

    expect(targets).toHaveLength(2);
    expect(targets[0]).toEqual(targets[1]);
    expect(targets[0]).toEqual(buildObjectContextMenuTarget(objectRow));
    expect(targets[0]).toMatchObject({
      id: '1',
      isEnabled: true,
      isPinned: true,
    });
  });

  it('masks object mutation capabilities when source is unavailable', () => {
    const mockVirtualizerFactory = () => ({
      getTotalSize: () => 70,
      getVirtualItems: () => [{ index: 0, size: 70, start: 0 }],
    });

    render(
      <ObjectListContent
        parentRef={{ current: null }}
        rowVirtualizer={
          mockVirtualizerFactory() as unknown as import('@tanstack/react-virtual').Virtualizer<
            HTMLDivElement,
            Element
          >
        }
        flatObjectItems={[{ type: 'row', obj: objectRow }]}
        selectedObjectFolderPath={null}
        selectedObjectType={null}
        onSelectObject={vi.fn()}
        setSelectedObjectType={vi.fn()}
        isMobile={false}
        stickyPosition={null}
        selectedIndex={-1}
        scrollToSelected={vi.fn()}
        mutationsDisabled
        contextMenuProps={{
          isSyncing: false,
          categoryNames: [{ name: 'Character', label: 'Characters' }],
          handleEdit: vi.fn(),
          handleSyncWithDb: vi.fn(),
          handleDeleteObject: vi.fn(),
          handlePin: vi.fn(),
          handleMoveCategory: vi.fn(),
          handleRevealInExplorer: vi.fn(),
          handleEnableObject: vi.fn(),
          handleDisableObject: vi.fn(),
        }}
      />,
    );

    const target = JSON.parse(screen.getByTestId('object-context-target').textContent ?? '{}');
    expect(target.capabilities.can_toggle).toBe(false);
    expect(target.capabilities.can_delete).toBe(false);
    expect(target.capabilities.can_sync).toBe(false);
  });
});
