import { render, screen, fireEvent } from '../../testing/test-utils';
import FolderCard from './FolderCard';
import { beforeEach, vi, describe, it, expect } from 'vitest';
import type { WorkspaceCapabilities, WorkspaceExplorerNode } from '../../types/workspace';
import { useAppStore } from '../../stores/useAppStore';

// Mock dependencies
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  convertFileSrc: vi.fn((path) => `asset://${path}`),
}));

vi.mock('@tauri-apps/plugin-fs', () => ({
  readFile: vi.fn(),
  readDir: vi.fn(),
  exists: vi.fn(),
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
  ask: vi.fn(),
  message: vi.fn(),
}));

// Mock ContextMenu to just render children
vi.mock('../../components/ui/ContextMenu', () => ({
  ContextMenu: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
  ContextMenuItem: () => null,
  ContextMenuSeparator: () => null,
}));

vi.mock('./FolderCardContextMenu', () => ({
  default: () => null,
}));

vi.mock('./BulkContextMenu', () => ({
  default: () => null,
}));

const baseCapabilities: WorkspaceCapabilities = {
  can_toggle: true,
  can_rename: true,
  can_delete: true,
  can_move: false,
  can_toggle_safe: false,
  can_sync: false,
  can_enable_only_this: false,
  can_pin: false,
  can_edit_metadata: false,
  can_reveal_in_explorer: true,
  can_move_category: false,
  can_open_in_explorer: true,
};

const mockFolder: WorkspaceExplorerNode = {
  node_type: 'ContainerFolder',
  classification_reasons: [],
  name: 'Test Mod',
  folder_name: 'Test Mod',
  path: '/mods/Test Mod',
  is_enabled: true,
  is_directory: true,
  thumbnail_path: null,
  modified_at: 1234567890,
  size_bytes: 1024,
  has_info_json: true,
  is_favorite: false,
  is_misplaced: false,
  is_safe: true,
  metadata: null,
  category: null,
  warnings: [],
  node_kind: 'container',
  display_mode: 'container_folder',
  type_chip: null,
  display_name: 'Test Mod',
  is_effectively_active: true,
  ancestor_disabled: false,
  inactive_reason: null,
  warning_state: 'none',
  primary_warning: null,
  switch_state: 'enabled',
  switch_reason: null,
  switch_policy_key: 'mod',
  capabilities: baseCapabilities,
  can_navigate: true,
};

describe('FolderCard', () => {
  beforeEach(() => {
    useAppStore.setState({ safeMode: false });
  });

  it('renders mod name', () => {
    render(
      <FolderCard
        folder={mockFolder}
        isSelected={false}
        onNavigate={vi.fn()}
        toggleSelection={vi.fn()}
      />,
    );
    expect(screen.getByText('Test Mod')).toBeInTheDocument();
  });

  it('handles click events', () => {
    const toggleSelection = vi.fn();
    const onActivate = vi.fn();

    render(
      <FolderCard
        folder={mockFolder}
        isSelected={false}
        onNavigate={vi.fn()}
        toggleSelection={toggleSelection}
        onActivate={onActivate}
      />,
    );

    fireEvent.click(screen.getByText('Test Mod'));
    expect(onActivate).toHaveBeenCalledWith(mockFolder.path);
    expect(toggleSelection).not.toHaveBeenCalled();
  });

  it('handles ctrl+click', () => {
    const toggleSelection = vi.fn();

    render(
      <FolderCard
        folder={mockFolder}
        isSelected={false}
        onNavigate={vi.fn()}
        toggleSelection={toggleSelection}
      />,
    );

    fireEvent.click(screen.getByText('Test Mod'), { ctrlKey: true });
    expect(toggleSelection).toHaveBeenCalledWith(mockFolder.path, true, false);
  });

  it('does not select InternalAssets on click', () => {
    const toggleSelection = vi.fn();
    const onActivate = vi.fn();
    const internalAssetFolder = {
      ...mockFolder,
      node_type: 'InternalAssets',
      display_mode: 'internal_assets',
      can_navigate: false,
    } as WorkspaceExplorerNode;

    render(
      <FolderCard
        folder={internalAssetFolder}
        isSelected={false}
        onNavigate={vi.fn()}
        toggleSelection={toggleSelection}
        onActivate={onActivate}
      />,
    );

    fireEvent.click(screen.getByText('Test Mod'));
    expect(onActivate).not.toHaveBeenCalled();
    expect(toggleSelection).not.toHaveBeenCalled();
  });

  it('handles double click', () => {
    const onNavigate = vi.fn();
    render(
      <FolderCard
        folder={mockFolder}
        isSelected={false}
        onNavigate={onNavigate}
        toggleSelection={vi.fn()}
      />,
    );

    const card = screen.getByRole('gridcell');
    fireEvent.doubleClick(card);
    expect(onNavigate).toHaveBeenCalledWith(mockFolder.folder_name);
  });

  it('reflects enabled/disabled state visually (TC-13)', () => {
    const { rerender } = render(
      <FolderCard
        folder={mockFolder}
        isSelected={false}
        onNavigate={vi.fn()}
        toggleSelection={vi.fn()}
      />,
    );
    expect(screen.getByText('Enabled')).toBeInTheDocument();
    expect(screen.getAllByRole('checkbox', { hidden: true })[1]).toBeChecked();

    const disabledFolder = {
      ...mockFolder,
      is_enabled: false,
      switch_state: 'disabled' as const,
    };
    rerender(
      <FolderCard
        folder={disabledFolder}
        isSelected={false}
        onNavigate={vi.fn()}
        toggleSelection={vi.fn()}
      />,
    );
    expect(screen.getByText('Disabled')).toBeInTheDocument();
    expect(screen.getAllByRole('checkbox', { hidden: true })[1]).not.toBeChecked();
  });

  it('disables switch mutations when source is unavailable', () => {
    const onToggleEnabled = vi.fn();

    render(
      <FolderCard
        folder={mockFolder}
        isSelected={false}
        onNavigate={vi.fn()}
        toggleSelection={vi.fn()}
        onToggleEnabled={onToggleEnabled}
        mutationsDisabled
      />,
    );

    const switchControl = screen.getAllByRole('checkbox', { hidden: true })[1];
    expect(switchControl).toBeDisabled();
    fireEvent.click(switchControl);
    expect(onToggleEnabled).not.toHaveBeenCalled();
  });

  it('renders naming conflict warning styles (TC-13)', () => {
    const conflictFolder = { ...mockFolder, conflict_state: 'both' } as WorkspaceExplorerNode;
    render(
      <FolderCard
        folder={conflictFolder}
        isSelected={false}
        onNavigate={vi.fn()}
        toggleSelection={vi.fn()}
        hasConflict={false}
      />,
    );

    const card = screen.getByRole('gridcell');
    expect(card).toHaveClass('border-warning/60');
    expect(card).toHaveClass('ring-warning/40');
  });

  it('TC-21-01: renders Rename input and submits on Enter', () => {
    const onRenameSubmit = vi.fn();
    const onRenameCancel = vi.fn();

    render(
      <FolderCard
        folder={mockFolder}
        isSelected={false}
        onNavigate={vi.fn()}
        toggleSelection={vi.fn()}
        isRenaming={true}
        onRenameSubmit={onRenameSubmit}
        onRenameCancel={onRenameCancel}
      />,
    );

    const input = screen.getByDisplayValue('Test Mod');
    fireEvent.change(input, { target: { value: 'BetterName' } });
    fireEvent.keyDown(input, { key: 'Enter', code: 'Enter' });

    expect(onRenameSubmit).toHaveBeenCalledWith('BetterName');
    expect(onRenameCancel).not.toHaveBeenCalled();
  });

  it('TC-21: cancels rename on Escape or Blur', () => {
    const onRenameSubmit = vi.fn();
    const onRenameCancel = vi.fn();

    render(
      <FolderCard
        folder={mockFolder}
        isSelected={false}
        onNavigate={vi.fn()}
        toggleSelection={vi.fn()}
        isRenaming={true}
        onRenameSubmit={onRenameSubmit}
        onRenameCancel={onRenameCancel}
      />,
    );

    const input = screen.getByDisplayValue('Test Mod');
    fireEvent.keyDown(input, { key: 'Escape', code: 'Escape' });
    expect(onRenameCancel).toHaveBeenCalledTimes(1);

    // Test onBlur
    fireEvent.blur(input);
    expect(onRenameCancel).toHaveBeenCalledTimes(2);
    expect(onRenameSubmit).not.toHaveBeenCalled();
  });

  it('masks unsafe folder names while safe mode leak guard is active', () => {
    useAppStore.setState({ safeMode: true });
    const unsafeFolder = {
      ...mockFolder,
      is_safe: false,
      name: 'Unsafe Mod Name',
      display_name: 'Unsafe Mod Name',
      folder_name: 'Unsafe Mod Name',
    };

    render(
      <FolderCard
        folder={unsafeFolder}
        isSelected={false}
        onNavigate={vi.fn()}
        toggleSelection={vi.fn()}
      />,
    );

    const maskedName = screen.getByText('[Hidden Mod]');
    expect(maskedName).toBeInTheDocument();
    expect(maskedName).toHaveClass('blur-xs');
    expect(screen.queryByText('Unsafe Mod Name')).not.toBeInTheDocument();
  });
});
