import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { ObjectContextMenu, type ContextMenuTarget } from './ObjectContextMenu';

// Mock inner components to simplify
vi.mock('../../components/ui/ContextMenu', () => ({
  ContextMenuItem: ({ children, onClick }: { children: React.ReactNode; onClick: () => void }) => (
    <button onClick={onClick}>{children}</button>
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

  it('renders object menu correctly', () => {
    const itemTarget: ContextMenuTarget = {
      type: 'object',
      id: '1',
      name: 'Zeta',
      objectType: 'Character',
      isEnabled: true,
      isPinned: false,
    };
    const onPin = vi.fn();

    render(
      <ObjectContextMenu
        item={itemTarget}
        isSyncing={false}
        categories={dummyCategories}
        onEditObject={vi.fn()}
        onEditFolder={vi.fn()}
        onSyncWithDb={vi.fn()}
        onDelete={vi.fn()}
        onDeleteObject={vi.fn()}
        onToggle={vi.fn()}
        onOpen={vi.fn()}
        onPin={onPin}
        onFavorite={vi.fn()}
        onMoveCategory={vi.fn()}
      />,
    );

    expect(screen.getByText('Edit Metadata')).toBeInTheDocument();
    expect(screen.getByText('Pin to Top')).toBeInTheDocument();

    fireEvent.click(screen.getByText('Pin to Top'));
    expect(onPin).toHaveBeenCalledWith('1');
  });

  it('renders folder menu correctly', () => {
    const folderTarget: ContextMenuTarget = {
      type: 'folder',
      path: 'C:\\mods',
      name: 'Mods',
      isEnabled: true,
    };
    const onToggle = vi.fn();

    render(
      <ObjectContextMenu
        item={folderTarget}
        isSyncing={false}
        categories={dummyCategories}
        onEditObject={vi.fn()}
        onEditFolder={vi.fn()}
        onSyncWithDb={vi.fn()}
        onDelete={vi.fn()}
        onDeleteObject={vi.fn()}
        onToggle={onToggle}
        onOpen={vi.fn()}
        onPin={vi.fn()}
        onFavorite={vi.fn()}
        onMoveCategory={vi.fn()}
      />,
    );

    expect(screen.getByText('Disable')).toBeInTheDocument(); // Since isEnabled is true
    fireEvent.click(screen.getByText('Disable'));
    expect(onToggle).toHaveBeenCalledWith('C:\\mods', true);
  });
});
