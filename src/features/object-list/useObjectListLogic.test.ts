import { renderHook, act } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { useObjectListLogic } from './useObjectListLogic';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import React from 'react';

vi.mock('../../stores/useAppStore', () => ({
  useAppStore: Object.assign(
    vi.fn(() => ({
      selectedObject: null,
      setSelectedObject: vi.fn(),
      selectedObjectType: null,
      setSelectedObjectType: vi.fn(),
      sidebarSearchQuery: '',
      setSidebarSearch: vi.fn(),
      safeMode: false,
      setSafeMode: vi.fn(),
    })),
    {
      getState: vi.fn(() => ({
        explorerSubPath: '',
        setExplorerSubPath: vi.fn(),
        setCurrentPath: vi.fn(),
      })),
    },
  ),
}));

vi.mock('../../hooks/useObjects', () => ({
  useObjects: vi.fn(() => ({ data: [], isLoading: false, isError: false })),
  useGameSchema: vi.fn(() => ({ data: undefined })),
}));

vi.mock('../../hooks/useActiveGame', () => ({
  useActiveGame: vi.fn(() => ({ activeGame: null })),
}));

vi.mock('../../hooks/useResponsive', () => ({
  useResponsive: vi.fn(() => ({ isMobile: false })),
}));

vi.mock('./hooks/useSearchWorker', () => ({
  useSearchWorker: vi.fn(() => ({ filteredIds: null, search: vi.fn() })),
}));

vi.mock('./useObjectListHandlers', () => ({
  useObjectListHandlers: vi.fn(() => ({
    deleteDialog: { open: false, path: '', name: '', itemCount: 0 },
    setDeleteDialog: vi.fn(),
    handleToggle: vi.fn(),
    handleOpen: vi.fn(),
    handleDelete: vi.fn(),
    confirmDelete: vi.fn(),
    handleDeleteObject: vi.fn(),
    editObject: null,
    setEditObject: vi.fn(),
    handleEdit: vi.fn(),
    handleSync: vi.fn(),
    isSyncing: false,
    handleSyncWithDb: vi.fn(),
    handleApplySyncMatch: vi.fn(),
    syncConfirm: { open: false },
    setSyncConfirm: vi.fn(),
    scanReview: { open: false },
    handleCommitScan: vi.fn(),
    handleCloseScanReview: vi.fn(),
    handlePin: vi.fn(),
    handleFavorite: vi.fn(),
    handleMoveCategory: vi.fn(),
    handleRevealInExplorer: vi.fn(),
    handleEnableObject: vi.fn(),
    handleDisableObject: vi.fn(),
    categoryNames: [],
    handleDropOnItem: vi.fn(),
    handleDropAutoOrganize: vi.fn(),
    handleDropNewObject: vi.fn(),
    handleDropOnNewObjectSubmit: vi.fn(),
    archiveModal: { open: false },
    handleArchiveExtractSubmit: vi.fn(),
    handleArchiveExtractSkip: vi.fn(),
  })),
}));

vi.mock('./useObjectListVirtualizer', () => ({
  useObjectListVirtualizer: vi.fn(() => ({
    parentRef: { current: null },
    rowVirtualizer: {},
    flatObjectItems: [],
    totalItems: 0,
    stickyPosition: null,
    selectedIndex: -1,
    scrollToSelected: vi.fn(),
  })),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

const createWrapper = () => {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return ({ children }: { children: React.ReactNode }) =>
    React.createElement(QueryClientProvider, { client: queryClient }, children);
};

describe('useObjectListLogic', () => {
  it('initializes basic state correctly', () => {
    const { result } = renderHook(() => useObjectListLogic(), { wrapper: createWrapper() });

    expect(result.current.isMobile).toBe(false);
    expect(result.current.objects).toEqual([]);
    expect(result.current.activeFilters).toEqual({});
    expect(result.current.sortBy).toBe('name');
  });

  it('updates filters properly', () => {
    const { result } = renderHook(() => useObjectListLogic(), { wrapper: createWrapper() });

    act(() => {
      result.current.handleFilterChange('element', ['Fire']);
    });

    expect(result.current.activeFilters).toEqual({ element: ['Fire'] });

    act(() => {
      result.current.handleClearFilters();
    });

    expect(result.current.activeFilters).toEqual({});
  });
});
