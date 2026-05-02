import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { ObjectContextMenu, type ContextMenuTarget } from './ObjectContextMenu';
import type { WorkspaceCapabilities } from '../../types/workspace';

// Mock inner components to simplify
vi.mock('../../components/ui/ContextMenu', () => ({
  ContextMenuItem: ({
    children,
    onClick,
    disabled,
  }: {
    children: React.ReactNode;
    onClick?: () => void;
    disabled?: boolean;
  }) => (
    <button onClick={onClick} disabled={disabled}>
      {children}
    </button>
  ),
  ContextMenuSeparator: () => <hr />,
  ContextMenuSub: ({ children, label }: { children: React.ReactNode; label: React.ReactNode }) => (
    <div>
      {label}
      <div>{children}</div>
    </div>
  ),
}));

describe('ObjectContextMenu', () => {
  const dummyCategories = [{ name: 'Character', label: 'Characters' }];
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
  const createHandlers = () => ({
    onEditObject: vi.fn(),
    onSyncWithDb: vi.fn(),
    onDeleteObject: vi.fn(),
    onPin: vi.fn(),
    onMoveCategory: vi.fn(),
    onRevealInExplorer: vi.fn(),
    onEnableObject: vi.fn(),
    onDisableObject: vi.fn(),
  });

  it('renders unpinned object menu and wires actions', () => {
    const handlers = createHandlers();
    const itemTarget: ContextMenuTarget = {
      type: 'object',
      id: '1',
      name: 'Zeta',
      objectType: 'Character',
      isEnabled: true,
      enabledCount: 5,
      modCount: 10,
      isPinned: false,
      category: 'Character',
      capabilities: baseCapabilities,
    };

    render(
      <ObjectContextMenu
        item={itemTarget}
        isSyncing={false}
        categories={dummyCategories}
        {...handlers}
      />,
    );

    expect(screen.getByText('context.edit_meta')).toBeInTheDocument();
    expect(screen.getByText('context.reveal_explorer')).toBeInTheDocument();
    expect(screen.getByText('context.pin_top')).toBeInTheDocument();
    expect(screen.getByText('context.disable')).toBeInTheDocument();
    expect(screen.queryByText('context.enable')).toBeNull();
    expect(screen.getByText('context.move_category')).toBeInTheDocument();
    expect(screen.getByText('Characters')).toBeInTheDocument();
    expect(screen.getByText('context.sync_db')).toBeInTheDocument();
    expect(screen.getByText('context.delete_object')).toBeInTheDocument();

    fireEvent.click(screen.getByText('context.edit_meta'));
    fireEvent.click(screen.getByText('context.reveal_explorer'));
    fireEvent.click(screen.getByText('context.pin_top'));
    fireEvent.click(screen.getByText('context.disable'));
    fireEvent.click(screen.getByText('Characters'));
    fireEvent.click(screen.getByText('context.sync_db'));
    fireEvent.click(screen.getByText('context.delete_object'));

    expect(handlers.onEditObject).toHaveBeenCalledWith('1');
    expect(handlers.onRevealInExplorer).toHaveBeenCalledWith('1');
    expect(handlers.onPin).toHaveBeenCalledWith('1');
    expect(handlers.onDisableObject).toHaveBeenCalledWith('1');
    expect(handlers.onMoveCategory).toHaveBeenCalledWith('1', 'Character', 'object');
    expect(handlers.onSyncWithDb).toHaveBeenCalledWith('1', 'Zeta');
    expect(handlers.onDeleteObject).toHaveBeenCalledWith('1');
  });

  it('renders pinned disabled object menu with enable action only', () => {
    const handlers = createHandlers();
    const itemTarget: ContextMenuTarget = {
      type: 'object',
      id: '2',
      name: 'Amber',
      objectType: 'Character',
      isEnabled: false,
      enabledCount: 0,
      modCount: 4,
      isPinned: true,
      category: 'Character',
      capabilities: baseCapabilities,
    };

    render(
      <ObjectContextMenu
        item={itemTarget}
        isSyncing={false}
        categories={dummyCategories}
        {...handlers}
      />,
    );

    expect(screen.getByText('context.unpin')).toBeInTheDocument();
    expect(screen.getByText('context.enable')).toBeInTheDocument();
    expect(screen.queryByText('context.disable')).toBeNull();

    fireEvent.click(screen.getByText('context.enable'));
    expect(handlers.onEnableObject).toHaveBeenCalledWith('2');
  });

  it('disables sync action while syncing', () => {
    const handlers = createHandlers();
    const itemTarget: ContextMenuTarget = {
      type: 'object',
      id: '3',
      name: 'Kaeya',
      objectType: 'Character',
      isEnabled: true,
      enabledCount: 1,
      modCount: 3,
      isPinned: false,
      category: 'Character',
      capabilities: baseCapabilities,
    };

    render(
      <ObjectContextMenu item={itemTarget} isSyncing categories={dummyCategories} {...handlers} />,
    );

    const syncButton = screen.getByRole('button', { name: 'context.syncing' });
    expect(syncButton).toBeDisabled();
    fireEvent.click(syncButton);
    expect(handlers.onSyncWithDb).not.toHaveBeenCalled();
  });
});
