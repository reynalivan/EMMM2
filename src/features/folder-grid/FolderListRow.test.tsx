import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import FolderListRow from './FolderListRow';
import type { WorkspaceCapabilities, WorkspaceExplorerNode } from '../../types/workspace';

vi.mock('../../hooks/useThumbnail', () => ({
  useThumbnail: vi.fn((_gameId: string, _path: string) => ({ data: null, isLoading: false })),
}));
vi.mock('../../hooks/useModContextMenuItems', () => ({
  useModContextMenuItems: () => [],
}));
vi.mock('../mod-runtime/actions/useModContextMenuActions', () => ({
  useModContextMenuActions: () => ({
    openExplorer: vi.fn(),
    pasteThumbnailFromClipboard: vi.fn(),
    importThumbnail: vi.fn(),
  }),
}));
vi.mock('@tauri-apps/api/core', () => ({
  convertFileSrc: vi.fn(),
}));

// ContextMenu is wrapped
vi.mock('../../components/ui/ContextMenu', () => ({
  ContextMenu: ({ children, content }: { children: React.ReactNode; content: React.ReactNode }) => (
    <div data-testid="context-menu-wrapper">
      {children}
      <div data-testid="context-menu-content" className="hidden">
        {content}
      </div>
    </div>
  ),
  ContextMenuItem: ({ children, onClick }: { children: React.ReactNode; onClick: () => void }) => (
    <button onClick={onClick}>{children}</button>
  ),
  ContextMenuSeparator: () => <hr />,
}));

describe('FolderListRow', () => {
  const baseCapabilities: WorkspaceCapabilities = {
    can_toggle: true,
    can_rename: true,
    can_delete: true,
    can_move: true,
    can_toggle_safe: true,
    can_sync: true,
    can_enable_only_this: false,
    can_pin: true,
    can_edit_metadata: false,
    can_reveal_in_explorer: true,
    can_move_category: false,
    can_open_in_explorer: true,
  };

  const dummyFolder: WorkspaceExplorerNode = {
    path: 'C:\\mods\\folder1',
    name: 'Folder 1',
    folder_name: 'Folder 1',
    is_enabled: true,
    is_favorite: false,
    is_safe: true,
    node_type: 'ModPackRoot',
    is_directory: true,
    classification_reasons: [],
    thumbnail_path: null,
    modified_at: 0,
    size_bytes: 0,
    has_info_json: false,
    is_misplaced: false,
    metadata: null,
    category: null,
    warnings: [],
    node_kind: 'terminal_mod',
    display_mode: 'mod_pack',
    type_chip: 'mod_pack',
    display_name: 'Folder 1',
    is_effectively_active: true,
    ancestor_disabled: false,
    inactive_reason: null,
    warning_state: 'none',
    primary_warning: null,
    switch_state: 'enabled',
    switch_reason: null,
    switch_policy_key: 'mod',
    capabilities: baseCapabilities,
    can_navigate: false,
  };

  it('renders folder info correctly', () => {
    const toggleSelection = vi.fn();

    render(
      <FolderListRow
        item={dummyFolder}
        isSelected={false}
        toggleSelection={toggleSelection}
      />,
    );

    expect(screen.getByText('Folder 1')).toBeInTheDocument();
  });

  it('calls selection correctly on click', () => {
    const toggleSelection = vi.fn();
    const onActivate = vi.fn();

    render(
      <FolderListRow
        item={dummyFolder}
        isSelected={false}
        toggleSelection={toggleSelection}
        onActivate={onActivate}
      />,
    );

    fireEvent.click(screen.getByText('Folder 1'));
    expect(onActivate).toHaveBeenCalledWith('C:\\mods\\folder1');
    expect(toggleSelection).not.toHaveBeenCalled();
  });

  it('does not select InternalAssets on click', () => {
    const toggleSelection = vi.fn();
    const onActivate = vi.fn();

    render(
      <FolderListRow
        item={{
          ...dummyFolder,
          node_type: 'InternalAssets',
          display_mode: 'internal_assets',
          can_navigate: false,
        }}
        isSelected={false}
        toggleSelection={toggleSelection}
        onActivate={onActivate}
      />,
    );

    fireEvent.click(screen.getByText('Folder 1'));
    expect(onActivate).not.toHaveBeenCalled();
    expect(toggleSelection).not.toHaveBeenCalled();
  });
});
