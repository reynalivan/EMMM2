import { render, screen, fireEvent } from '../../test-utils';
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
    expect(toggleSelection).toHaveBeenCalledWith(mockFolder.path, false);
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
    expect(toggleSelection).toHaveBeenCalledWith(mockFolder.path, true);
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
});
