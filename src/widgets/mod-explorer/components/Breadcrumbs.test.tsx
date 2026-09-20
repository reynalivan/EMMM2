import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import { describe, it, expect, vi } from 'vitest';
import type { WorkspaceExplorerNode } from '@/entities/workspace';
import ExplorerBreadcrumbs from './Breadcrumbs';

vi.mock('@/shared/ui/liquid', () => ({
  LiquidSurface: ({ children }: { children: ReactNode }) => <div>{children}</div>,
}));

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

  it('collapses the leading path when the breadcrumb has limited width', async () => {
    class ResizeObserverStub {
      constructor(private readonly callback: ResizeObserverCallback) {}

      observe() {
        this.callback(
          [{ contentRect: { width: 200 } } as ResizeObserverEntry],
          this as unknown as ResizeObserver,
        );
      }

      disconnect() {}
      unobserve() {}
    }

    vi.stubGlobal('ResizeObserver', ResizeObserverStub);

    try {
      render(
        <ExplorerBreadcrumbs
          path={['Folder1', 'Folder2', 'Folder3', 'Folder4', 'Folder5']}
          onNavigate={vi.fn()}
          onGoHome={vi.fn()}
        />,
      );

      await waitFor(() => expect(screen.queryByText('Folder1')).toBeNull());
      expect(screen.getByText('…')).toBeInTheDocument();
      expect(screen.getByText('Folder5')).toBeInTheDocument();
    } finally {
      vi.unstubAllGlobals();
    }
  });

  it('shows searchable, scrollable sibling folders when hovering the previous breadcrumb', () => {
    const onNavigateToPreviousFolder = vi.fn();
    const folders = [
      {
        name: 'Aglaea',
        path: 'E:/Mods/SkinSelectImpact/Aglaea',
        can_navigate: true,
        is_favorite: false,
      },
      {
        name: 'Castorice',
        path: 'E:/Mods/SkinSelectImpact/Castorice',
        can_navigate: true,
        is_favorite: true,
      },
      {
        name: 'Readme',
        path: 'E:/Mods/SkinSelectImpact/Readme',
        can_navigate: false,
        is_favorite: false,
      },
    ] as WorkspaceExplorerNode[];

    render(
      <ExplorerBreadcrumbs
        path={['SkinSelectImpact', 'Current folder']}
        onNavigate={vi.fn()}
        onNavigateToPreviousFolder={onNavigateToPreviousFolder}
        previousFolderItems={folders}
        onGoHome={vi.fn()}
        isRootHidden
      />,
    );

    fireEvent.mouseEnter(screen.getByText('SkinSelectImpact'));

    const menu = screen.getByRole('dialog', { name: 'Folders in SkinSelectImpact' });
    expect(menu).toBeInTheDocument();
    expect(menu.parentElement?.parentElement).toHaveClass('pt-2');
    expect(screen.getByText('Aglaea')).toBeInTheDocument();
    expect(screen.getByText('Castorice')).toBeInTheDocument();
    expect(screen.getByText('Readme')).toBeInTheDocument();
    expect(Array.from(menu.querySelectorAll('button')).map((button) => button.textContent)).toEqual(
      ['Castorice', 'Aglaea', 'Readme'],
    );
    expect(menu.querySelector('.max-h-72.overflow-y-auto')).not.toBeNull();

    fireEvent.change(screen.getByRole('searchbox'), { target: { value: 'cast' } });
    expect(screen.queryByText('Aglaea')).toBeNull();
    fireEvent.click(screen.getByText('Castorice'));
    expect(onNavigateToPreviousFolder).toHaveBeenCalledWith('Castorice');
  });

  it('loads sibling continuation pages while the previous-folder menu remains open', async () => {
    const loadMorePreviousFolders = vi.fn().mockResolvedValue(undefined);
    const firstPage = [
      {
        name: 'Aglaea',
        path: 'E:/Mods/SkinSelectImpact/Aglaea',
        is_favorite: false,
      },
    ] as WorkspaceExplorerNode[];
    const secondPage = [
      ...firstPage,
      {
        name: 'Castorice',
        path: 'E:/Mods/SkinSelectImpact/Castorice',
        is_favorite: false,
      },
    ] as WorkspaceExplorerNode[];
    const baseProps = {
      path: ['SkinSelectImpact', 'Current folder'],
      onNavigate: vi.fn(),
      onNavigateToPreviousFolder: vi.fn(),
      onGoHome: vi.fn(),
      isRootHidden: true,
      loadMorePreviousFolders,
    };
    const { rerender } = render(
      <ExplorerBreadcrumbs
        {...baseProps}
        previousFolderItems={firstPage}
        hasMorePreviousFolders
        isLoadingMorePreviousFolders={false}
      />,
    );

    fireEvent.mouseEnter(screen.getByText('SkinSelectImpact'));
    const list = screen.getByTestId('breadcrumb-previous-folder-list');
    Object.defineProperties(list, {
      scrollHeight: { value: 1_000 },
      clientHeight: { value: 200 },
      scrollTop: { value: 750, writable: true },
    });
    fireEvent.scroll(list);
    await waitFor(() => expect(loadMorePreviousFolders).toHaveBeenCalledTimes(1));

    rerender(
      <ExplorerBreadcrumbs
        {...baseProps}
        previousFolderItems={firstPage}
        hasMorePreviousFolders
        isLoadingMorePreviousFolders
      />,
    );
    rerender(
      <ExplorerBreadcrumbs
        {...baseProps}
        previousFolderItems={secondPage}
        hasMorePreviousFolders
        isLoadingMorePreviousFolders={false}
      />,
    );

    fireEvent.scroll(screen.getByTestId('breadcrumb-previous-folder-list'));
    await waitFor(() => expect(loadMorePreviousFolders).toHaveBeenCalledTimes(2));
    expect(screen.getByText('Castorice')).toBeInTheDocument();
  });
});
