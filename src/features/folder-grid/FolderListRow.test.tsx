import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import FolderListRow from './FolderListRow';
import type { ModFolder } from '../../types/mod';

vi.mock('../../hooks/useThumbnail', () => ({
  useThumbnail: vi.fn(() => ({ data: null, isLoading: false })),
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
  const dummyFolder: ModFolder = {
    path: 'C:\\mods\\folder1',
    name: 'Folder 1',
    is_enabled: true,
    is_favorite: false,
    is_safe: true,
    node_type: 'Unknown',
    is_directory: true,
  } as ModFolder;

  it('renders folder info correctly', () => {
    const toggleSelection = vi.fn();
    const clearSelection = vi.fn();

    render(
      <FolderListRow
        item={dummyFolder}
        isSelected={false}
        toggleSelection={toggleSelection}
        clearSelection={clearSelection}
      />,
    );

    expect(screen.getByText('Folder 1')).toBeInTheDocument();
  });

  it('calls selection correctly on click', () => {
    const toggleSelection = vi.fn();
    const clearSelection = vi.fn();

    render(
      <FolderListRow
        item={dummyFolder}
        isSelected={false}
        toggleSelection={toggleSelection}
        clearSelection={clearSelection}
      />,
    );

    fireEvent.click(screen.getByText('Folder 1'));
    // By default clearSelection is called then toggleSelection
    expect(clearSelection).toHaveBeenCalled();
    expect(toggleSelection).toHaveBeenCalledWith('C:\\mods\\folder1', false);
  });
});
