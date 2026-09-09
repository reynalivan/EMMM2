import { fireEvent, render, screen } from '../../../tests/testing/test-utils';
import { describe, expect, it, vi } from 'vitest';
import FolderGridToolbar from './FolderGridToolbar';

describe('FolderGridToolbar', () => {
  it('lets users select a complete sort field and direction from a dropdown', () => {
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
        visibleCount={2}
      />,
    );

    const sortSelect = screen.getByTestId('folder-sort');
    expect(sortSelect).toHaveValue('name:asc');

    fireEvent.change(sortSelect, { target: { value: 'modified_at:desc' } });

    expect(setSortField).toHaveBeenCalledWith('modified_at');
    expect(setSortOrder).toHaveBeenCalledWith('desc');
  });
});
