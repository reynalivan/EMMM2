import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import FolderCardContextMenu from './FolderCardContextMenu';
import type { WorkspaceCapabilities, WorkspaceExplorerNode } from '../../types/workspace';

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
  ShieldCheck: () => <div data-testid="icon-shield-check" />,
  ShieldOff: () => <div data-testid="icon-shield-off" />,
  ClipboardPaste: () => <div data-testid="icon-clipboard-paste" />,
}));

// Mock custom hooks
vi.mock('../../hooks/useFolderMutations', () => ({
  usePasteThumbnail: () => ({ mutateAsync: vi.fn() }),
}));
vi.mock('../../hooks/useModContextMenuItems', () => ({
  useModContextMenuItems: (props: {
    folder: { is_enabled: boolean; is_favorite: boolean };
    onOpenMoveDialog?: (folder: WorkspaceExplorerNode) => void;
    onEnableOnlyThis?: () => void;
    onToggleFavorite: () => void;
    onDelete: () => void;
    onRename: () => void;
    onToggleEnabled: () => void;
  }) => {
    const items = [
      {
        id: 'rename',
        label: 'Rename',
        icon: () => null,
        onClick: props.onRename,
      },
      {
        id: 'toggle-enabled',
        label: props.folder.is_enabled ? 'Disable' : 'Enable',
        icon: () => null,
        onClick: props.onToggleEnabled,
      },
      {
        id: 'favorite',
        label: props.folder.is_favorite ? 'Unfavorite' : 'Favorite',
        icon: () => null,
        onClick: props.onToggleFavorite,
      },
      {
        id: 'delete',
        label: 'Delete',
        icon: () => null,
        onClick: props.onDelete,
      },
    ];

    if (props.onOpenMoveDialog) {
      items.push({
        id: 'move',
        label: 'Move to Object...',
        icon: () => null,
        onClick: () => props.onOpenMoveDialog?.(mockFolder),
      });
    }

    if (props.onEnableOnlyThis && !props.folder.is_enabled) {
      items.push({
        id: 'enable-only-this',
        label: 'Enable Only This',
        icon: () => null,
        onClick: props.onEnableOnlyThis,
      });
    }

    return items;
  },
}));

vi.mock('../mod-runtime/actions/useModContextMenuActions', () => ({
  useModContextMenuActions: () => ({
    openExplorer: vi.fn(),
    pasteThumbnailFromClipboard: vi.fn(),
    importThumbnail: vi.fn(),
  }),
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

const baseCapabilities: WorkspaceCapabilities = {
  can_toggle: true,
  can_rename: true,
  can_delete: true,
  can_move: true,
  can_toggle_safe: true,
  can_sync: true,
  can_enable_only_this: true,
  can_pin: true,
  can_edit_metadata: true,
  can_reveal_in_explorer: true,
  can_move_category: false,
  can_open_in_explorer: true,
};

const mockFolder: WorkspaceExplorerNode = {
  path: 'E:\\Mods\\Char\\Ayaka',
  folder_name: 'Ayaka',
  name: 'Ayaka Mod',
  node_type: 'VariantContainer',
  classification_reasons: [],
  is_enabled: false,
  is_favorite: false,
  is_safe: true,
  is_directory: true,
  thumbnail_path: null,
  modified_at: 0,
  size_bytes: 0,
  has_info_json: false,
  is_misplaced: false,
  metadata: null,
  category: null,
  warnings: [],
  node_kind: 'terminal_mod',
  display_mode: 'variant',
  type_chip: 'variant',
  display_name: 'Ayaka Mod',
  is_effectively_active: false,
  ancestor_disabled: false,
  inactive_reason: null,
  warning_state: 'none',
  primary_warning: null,
  switch_state: 'disabled',
  switch_reason: null,
  switch_policy_key: 'mod',
  capabilities: baseCapabilities,
  can_navigate: false,
};

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
