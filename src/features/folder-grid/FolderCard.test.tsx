import { render, screen, fireEvent } from '../../testing/test-utils';
import FolderCard from './FolderCard';
import { vi, describe, it, expect } from 'vitest';
import { ModFolder } from '../../types/mod';

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

const mockFolder: ModFolder = {
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
};

describe('FolderCard', () => {
  it('renders mod name', () => {
    render(
      <FolderCard
        folder={mockFolder}
        isSelected={false}
        onNavigate={vi.fn()}
        toggleSelection={vi.fn()}
        clearSelection={vi.fn()}
      />,
    );
    expect(screen.getByText('Test Mod')).toBeInTheDocument();
  });

  it('handles click events', () => {
    const toggleSelection = vi.fn();
    const clearSelection = vi.fn();

    render(
      <FolderCard
        folder={mockFolder}
        isSelected={false}
        onNavigate={vi.fn()}
        toggleSelection={toggleSelection}
        clearSelection={clearSelection}
      />,
    );

    fireEvent.click(screen.getByText('Test Mod'));
    expect(clearSelection).toHaveBeenCalled(); // Single click clears then toggles
    expect(toggleSelection).toHaveBeenCalledWith(mockFolder.path, false, false);
  });

  it('handles ctrl+click', () => {
    const toggleSelection = vi.fn();
    const clearSelection = vi.fn();

    render(
      <FolderCard
        folder={mockFolder}
        isSelected={false}
        onNavigate={vi.fn()}
        toggleSelection={toggleSelection}
        clearSelection={clearSelection}
      />,
    );

    fireEvent.click(screen.getByText('Test Mod'), { ctrlKey: true });
    expect(clearSelection).not.toHaveBeenCalled();
    expect(toggleSelection).toHaveBeenCalledWith(mockFolder.path, true, false);
  });

  it('handles double click', () => {
    const onNavigate = vi.fn();
    render(
      <FolderCard
        folder={mockFolder}
        isSelected={false}
        onNavigate={onNavigate}
        toggleSelection={vi.fn()}
        clearSelection={vi.fn()}
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
        clearSelection={vi.fn()}
      />,
    );
    expect(screen.getByText('Enabled')).toBeInTheDocument();
    expect(screen.getByRole('checkbox', { hidden: true })).toBeChecked();

    const disabledFolder = { ...mockFolder, is_enabled: false };
    rerender(
      <FolderCard
        folder={disabledFolder}
        isSelected={false}
        onNavigate={vi.fn()}
        toggleSelection={vi.fn()}
        clearSelection={vi.fn()}
      />,
    );
    expect(screen.getByText('Disabled')).toBeInTheDocument();
    expect(screen.getByRole('checkbox', { hidden: true })).not.toBeChecked();
  });

  it('renders naming conflict warning styles (TC-13)', () => {
    const conflictFolder = { ...mockFolder, conflict_state: 'both' } as unknown as ModFolder;
    render(
      <FolderCard
        folder={conflictFolder}
        isSelected={false}
        onNavigate={vi.fn()}
        toggleSelection={vi.fn()}
        clearSelection={vi.fn()}
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
        clearSelection={vi.fn()}
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
        clearSelection={vi.fn()}
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
});
