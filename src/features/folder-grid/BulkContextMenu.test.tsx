import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import BulkContextMenu from './BulkContextMenu';

vi.mock('../../components/ui/ContextMenu', () => ({
  ContextMenuItem: ({ children, onClick }: { children: React.ReactNode; onClick: () => void }) => (
    <button onClick={onClick}>{children}</button>
  ),
  ContextMenuSeparator: () => <hr />,
}));

describe('BulkContextMenu', () => {
  it('renders count correctly and triggers all callbacks', () => {
    const onToggle = vi.fn();
    const onDelete = vi.fn();
    const onTag = vi.fn();
    const onFavorite = vi.fn();
    const onSafe = vi.fn();
    const onPin = vi.fn();
    const onMoveToObject = vi.fn();

    render(
      <BulkContextMenu
        count={5}
        onToggle={onToggle}
        onDelete={onDelete}
        onTag={onTag}
        onFavorite={onFavorite}
        onSafe={onSafe}
        onPin={onPin}
        onMoveToObject={onMoveToObject}
      />,
    );

    expect(screen.getByText('5 items selected')).toBeInTheDocument();

    fireEvent.click(screen.getByText('Enable Selected'));
    expect(onToggle).toHaveBeenCalledWith(true);

    fireEvent.click(screen.getByText('Disable Selected'));
    expect(onToggle).toHaveBeenCalledWith(false);

    fireEvent.click(screen.getByText('Favorite Selected'));
    expect(onFavorite).toHaveBeenCalledWith(true);

    fireEvent.click(screen.getByText('Mark Safe'));
    expect(onSafe).toHaveBeenCalledWith(true);

    fireEvent.click(screen.getByText('Pin Selected'));
    expect(onPin).toHaveBeenCalledWith(true);

    fireEvent.click(screen.getByText('Add Tags...'));
    expect(onTag).toHaveBeenCalled();

    fireEvent.click(screen.getByText('Move to Object...'));
    expect(onMoveToObject).toHaveBeenCalled();

    fireEvent.click(screen.getByText('Delete 5 Items'));
    expect(onDelete).toHaveBeenCalled();
  });
});
