import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import ExplorerBreadcrumbs from './Breadcrumbs';

describe('ExplorerBreadcrumbs', () => {
  it('renders correctly with root hidden', () => {
    render(
      <ExplorerBreadcrumbs
        path={['A', 'B']}
        onNavigate={vi.fn()}
        onGoHome={vi.fn()}
        isRootHidden={true}
      />,
    );
    expect(screen.queryByText('ROOT')).toBeNull();
    expect(screen.getByText('A')).toBeInTheDocument();
    expect(screen.getByText('B')).toBeInTheDocument();
  });

  it('renders correctly with root visible', () => {
    const onGoHome = vi.fn();
    render(
      <ExplorerBreadcrumbs
        path={['A']}
        onNavigate={vi.fn()}
        onGoHome={onGoHome}
        isRootHidden={false}
      />,
    );
    const rootButton = screen.getByText('ROOT');
    expect(rootButton).toBeInTheDocument();
    fireEvent.click(rootButton);
    expect(onGoHome).toHaveBeenCalled();
  });

  it('truncates middle segments when path is too deep', () => {
    render(
      <ExplorerBreadcrumbs
        path={['Folder1', 'Folder2', 'Folder3', 'Folder4', 'Folder5']}
        onNavigate={vi.fn()}
        onGoHome={vi.fn()}
      />,
    );
    expect(screen.getByText('Folder1')).toBeInTheDocument();
    expect(screen.getByText('…')).toBeInTheDocument();
    expect(screen.queryByText('Folder2')).toBeNull();
    expect(screen.queryByText('Folder3')).toBeNull();
    expect(screen.getByText('Folder4')).toBeInTheDocument();
    expect(screen.getByText('Folder5')).toBeInTheDocument();
  });

  it('calls onNavigate with real index from truncated view', () => {
    const onNavigate = vi.fn();
    render(
      <ExplorerBreadcrumbs
        path={['F1', 'F2', 'F3', 'F4', 'F5']}
        onNavigate={onNavigate}
        onGoHome={vi.fn()}
      />,
    );
    // Clicking F5 should trigger onNavigate with full index 4
    fireEvent.click(screen.getByText('F5'));
    expect(onNavigate).toHaveBeenCalledWith(4);

    // Clicking F1 should trigger onNavigate with index 0
    fireEvent.click(screen.getByText('F1'));
    expect(onNavigate).toHaveBeenCalledWith(0);
  });
});
