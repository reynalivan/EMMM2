import { render, screen } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import ObjectListContent from './ObjectListContent';
import { buildObjectContextMenuTarget } from './ObjectContextMenuTarget';
import type { FlatItem } from './useObjectListVirtualizer';
import type { WorkspaceCapabilities, WorkspaceObjectNode } from '../../types/workspace';

vi.mock('../../components/ui/ContextMenu', () => ({
  ContextMenu: ({ children, content }: { children: React.ReactNode; content: React.ReactNode }) => (
    <div>
      <div data-testid="context-content">{content}</div>
      {children}
    </div>
  ),
}));
vi.mock('./ObjectRowItem', () => ({
  default: ({ obj }: { obj: { name: string } }) => <div data-testid="row-item">{obj.name}</div>,
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
    status: null,
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
      objectType: 'Character',
      category: 'Character',
      isEnabled: true,
      isPinned: true,
    });
  });
});
