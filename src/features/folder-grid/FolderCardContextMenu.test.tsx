import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import FolderCardContextMenu from './FolderCardContextMenu';
import type { ModFolder } from '../../types/mod';

// Mock Lucide icons
vi.mock('lucide-react', () => ({
  Star: () => <div data-testid="icon-star" />,
  ExternalLink: () => <div data-testid="icon-external-link" />,
  Pencil: () => <div data-testid="icon-pencil" />,
  Trash2: () => <div data-testid="icon-trash" />,
  ToggleLeft: () => <div data-testid="icon-toggle" />,
  Zap: () => <div data-testid="icon-zap" />,
  Image: () => <div data-testid="icon-image" />,
  ArrowRightLeft: () => <div data-testid="icon-arrow" />,
  FolderOpen: () => <div data-testid="icon-folder-open" />,
}));

// Mock custom hooks
vi.mock('../../hooks/useFolders', () => ({
  usePasteThumbnail: () => ({ mutateAsync: vi.fn() }),
}));

// Mock tauri plugins
vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
}));

vi.mock('@tauri-apps/plugin-fs', () => ({
  readFile: vi.fn(),
}));

vi.mock('../../components/ui/ContextMenu', () => ({
  ContextMenuItem: ({ children, onClick }: { children: React.ReactNode; onClick?: () => void }) => (
    <div role="menuitem" onClick={onClick}>
      {children}
    </div>
  ),
  ContextMenuSeparator: () => <hr role="separator" />,
}));

const mockFolder: ModFolder = {
  path: 'E:\\Mods\\Char\\Ayaka',
  folder_name: 'Ayaka',
  name: 'Ayaka Mod',
  game_id: 'genshin',
  is_enabled: false,
  is_favorite: false,
  is_safe: true,
  node_type: 'ModFolder',
  has_thumbnail: false,
  items: [],
  enabled_count: 0,
} as unknown as ModFolder;

describe('FolderCardContextMenu (TC-15)', () => {
  it('TC-15-009: Renders Move to Object when onOpenMoveDialog is provided', () => {
    const onOpenMoveDialog = vi.fn();
    render(
      <FolderCardContextMenu
        folder={mockFolder}
        onRename={vi.fn()}
        onDelete={vi.fn()}
        onToggle={vi.fn()}
        onToggleFavorite={vi.fn()}
        onOpenMoveDialog={onOpenMoveDialog}
      />,
    );

    // Check if the item exists
    const menuItems = screen.getAllByRole('menuitem');
    const moveItem = menuItems.find((item) => item.textContent?.includes('Move to Object...'));
    expect(moveItem).toBeDefined();

    fireEvent.click(moveItem!);
    expect(onOpenMoveDialog).toHaveBeenCalledWith(mockFolder);
  });

  it('TC-15-010: Renders Enable Only This when folder is disabled and onEnableOnlyThis is provided', () => {
    const onEnableOnlyThis = vi.fn();
    render(
      <FolderCardContextMenu
        folder={{ ...mockFolder, is_enabled: false }}
        onRename={vi.fn()}
        onDelete={vi.fn()}
        onToggle={vi.fn()}
        onToggleFavorite={vi.fn()}
        onEnableOnlyThis={onEnableOnlyThis}
      />,
    );

    const menuItems = screen.getAllByRole('menuitem');
    const enableItem = menuItems.find((item) => item.textContent?.includes('Enable Only This'));
    expect(enableItem).toBeDefined();

    fireEvent.click(enableItem!);
    expect(onEnableOnlyThis).toHaveBeenCalled();
  });

  it('TC-15-010: Does not render Enable Only This when folder is already enabled', () => {
    render(
      <FolderCardContextMenu
        folder={{ ...mockFolder, is_enabled: true }}
        onRename={vi.fn()}
        onDelete={vi.fn()}
        onToggle={vi.fn()}
        onToggleFavorite={vi.fn()}
        onEnableOnlyThis={vi.fn()}
      />,
    );

    const menuItems = screen.getAllByRole('menuitem');
    const enableItem = menuItems.find((item) => item.textContent?.includes('Enable Only This'));
    expect(enableItem).toBeUndefined();
  });

  it('TC-15-011: Renders Favorite sync option correctly', () => {
    const onToggleFavorite = vi.fn();
    const { rerender } = render(
      <FolderCardContextMenu
        folder={{ ...mockFolder, is_favorite: false }}
        onRename={vi.fn()}
        onDelete={vi.fn()}
        onToggle={vi.fn()}
        onToggleFavorite={onToggleFavorite}
      />,
    );

    let menuItems = screen.getAllByRole('menuitem');
    let favItem = menuItems.find((item) => item.textContent?.includes('Favorite'));
    expect(favItem).toBeDefined();
    fireEvent.click(favItem!);
    expect(onToggleFavorite).toHaveBeenCalled();

    rerender(
      <FolderCardContextMenu
        folder={{ ...mockFolder, is_favorite: true }}
        onRename={vi.fn()}
        onDelete={vi.fn()}
        onToggle={vi.fn()}
        onToggleFavorite={vi.fn()}
      />,
    );
    menuItems = screen.getAllByRole('menuitem');
    favItem = menuItems.find((item) => item.textContent?.includes('Unfavorite'));
    expect(favItem).toBeDefined();
  });
});
