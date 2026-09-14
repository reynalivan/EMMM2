import { fireEvent, render, screen } from '../../../tests/testing/test-utils';
import type { ReactNode } from 'react';
import { describe, expect, it, vi } from 'vitest';
import FolderGridToolbar from './FolderGridToolbar';

vi.mock('@/shared/ui/liquid', () => ({
  LiquidSurface: ({ children, className }: { children: ReactNode; className?: string }) => (
    <div className={className}>{children}</div>
  ),
}));

describe('FolderGridToolbar', () => {
  it('lets users select a complete sort field and direction from the glass menu', () => {
    const setSortField = vi.fn();
    const setSortOrder = vi.fn();

    render(
      <FolderGridToolbar
        isMobile={false}
        currentPath={[]}
        handleBreadcrumbClick={vi.fn()}
        previousFolderItems={[]}
        handleNavigate={vi.fn()}
        handleGoHome={vi.fn()}
        setMobilePane={vi.fn()}
        sortField="name"
        sortOrder="asc"
        setSortField={setSortField}
        setSortOrder={setSortOrder}
        viewMode="grid"
        setViewMode={vi.fn()}
        explorerSearchQuery=""
        setExplorerSearch={vi.fn()}
        canCreateFolder
        onCreateFolder={vi.fn()}
        isRefreshing
      />,
    );

    const sortMenu = screen.getByTestId('folder-sort');
    expect(sortMenu).toHaveAttribute('aria-expanded', 'false');

    fireEvent.click(sortMenu);
    fireEvent.click(screen.getByRole('option', { name: 'Date (Newest)' }));

    expect(setSortField).toHaveBeenCalledWith('modified_at');
    expect(setSortOrder).toHaveBeenCalledWith('desc');
  });

  it('marks breadcrumb and action regions for the pane-responsive toolbar layout', () => {
    render(
      <FolderGridToolbar
        isMobile={false}
        currentPath={[]}
        handleBreadcrumbClick={vi.fn()}
        previousFolderItems={[]}
        handleNavigate={vi.fn()}
        handleGoHome={vi.fn()}
        setMobilePane={vi.fn()}
        sortField="name"
        sortOrder="asc"
        setSortField={vi.fn()}
        setSortOrder={vi.fn()}
        viewMode="grid"
        setViewMode={vi.fn()}
        explorerSearchQuery=""
        setExplorerSearch={vi.fn()}
        canCreateFolder
        onCreateFolder={vi.fn()}
        isRefreshing
      />,
    );

    const controls = screen.getByTestId('folder-grid-toolbar-controls');

    expect(controls).toHaveClass('justify-end', 'w-full');
    expect(screen.getByTestId('folder-grid-toolbar-layout')).toHaveClass(
      'folder-grid-toolbar',
      'flex-col',
    );
    expect(screen.getByTestId('folder-grid-toolbar-breadcrumbs')).toHaveClass(
      'folder-grid-toolbar-breadcrumbs',
      'h-8',
    );
    expect(screen.getByRole('searchbox')).toBeInTheDocument();
    expect(screen.queryByText('5 items')).not.toBeInTheDocument();
    expect(screen.getByTestId('mod-grid-search')).toHaveClass('w-8');
    expect(screen.getByTestId('mod-grid-search-icon')).toHaveClass('z-10');
    expect(screen.getByTestId('add-folder')).toHaveClass('btn-square');
    expect(screen.getByTestId('folder-sort')).toHaveAttribute('aria-haspopup', 'listbox');
    expect(screen.getByRole('status', { name: 'Loading...' })).toBeInTheDocument();
    expect(
      screen.getByTestId('folder-grid-toolbar-controls').closest('.folder-grid-action-bar'),
    ).toBeTruthy();
    expect(screen.getByTestId('view-grid')).toBeInTheDocument();
    expect(screen.getByTestId('view-list')).toBeInTheDocument();
  });

  it('opens the add-folder dialog through the explicit toolbar action', () => {
    const onCreateFolder = vi.fn();

    render(
      <FolderGridToolbar
        isMobile={false}
        currentPath={[]}
        handleBreadcrumbClick={vi.fn()}
        previousFolderItems={[]}
        handleNavigate={vi.fn()}
        handleGoHome={vi.fn()}
        setMobilePane={vi.fn()}
        sortField="name"
        sortOrder="asc"
        setSortField={vi.fn()}
        setSortOrder={vi.fn()}
        viewMode="grid"
        setViewMode={vi.fn()}
        explorerSearchQuery=""
        setExplorerSearch={vi.fn()}
        canCreateFolder
        onCreateFolder={onCreateFolder}
      />,
    );

    fireEvent.click(screen.getByTestId('add-folder'));

    expect(onCreateFolder).toHaveBeenCalledOnce();
  });
});
