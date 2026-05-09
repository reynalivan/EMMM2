import { renderHook, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useObjectListLogic } from './useObjectListLogic';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import React from 'react';

const setObjectMetaFilters = vi.fn();

const appStoreState: {
  selectedObjectFolderPath: string | null;
  setSelectedObjectFolderPath: ReturnType<typeof vi.fn>;
  setSelectedModPath: ReturnType<typeof vi.fn>;
  selectedObjectType: string | null;
  setSelectedObjectType: ReturnType<typeof vi.fn>;
  sidebarSearchQuery: string;
  setSidebarSearch: ReturnType<typeof vi.fn>;
  setExplorerSubPath: ReturnType<typeof vi.fn>;
  setCurrentPath: ReturnType<typeof vi.fn>;
  clearGridSelection: ReturnType<typeof vi.fn>;
  setExplorerScrollOffset: ReturnType<typeof vi.fn>;
  safeMode: boolean;
  objectMetaFilters: Record<string, string[]>;
  setObjectMetaFilters: typeof setObjectMetaFilters;
  objectSortBy: 'name' | 'date' | 'rarity';
  setObjectSortBy: ReturnType<typeof vi.fn>;
  objectStatusFilter: 'all' | 'enabled' | 'disabled';
  setObjectStatusFilter: ReturnType<typeof vi.fn>;
} = {
  selectedObjectFolderPath: null,
  setSelectedObjectFolderPath: vi.fn(),
  setSelectedModPath: vi.fn(),
  selectedObjectType: null,
  setSelectedObjectType: vi.fn(),
  sidebarSearchQuery: '',
  setSidebarSearch: vi.fn(),
  setExplorerSubPath: vi.fn(),
  setCurrentPath: vi.fn(),
  clearGridSelection: vi.fn(),
  setExplorerScrollOffset: vi.fn(),
  safeMode: false,
  objectMetaFilters: {},
  setObjectMetaFilters,
  objectSortBy: 'name',
  setObjectSortBy: vi.fn(),
  objectStatusFilter: 'all',
  setObjectStatusFilter: vi.fn(),
};

vi.mock('../../stores/useAppStore', () => ({
  useAppStore: Object.assign(
    vi.fn(() => appStoreState),
    {
      getState: vi.fn(() => ({
        explorerSubPath: '',
        setExplorerSubPath: vi.fn(),
        setCurrentPath: vi.fn(),
      })),
    },
  ),
}));

const useGameSchemaMock = vi.fn<() => { data: unknown }>(() => ({ data: undefined }));
vi.mock('../../hooks/useObjectQueries', () => ({
  useGameSchema: () => useGameSchemaMock(),
}));

vi.mock('../workspace-runtime/useWorkspaceViewModel', () => ({
  useWorkspaceViewModel: vi.fn(() => ({ data: { objects: [] }, isLoading: false, isError: false })),
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
    handleMoveCategory: vi.fn(),
    handleRevealInExplorer: vi.fn(),
    handleEnableObject: vi.fn(),
    handleDisableObject: vi.fn(),
    categoryNames: [],
    handleDropOnItem: vi.fn(),
    handleDropAutoOrganize: vi.fn(),
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
  beforeEach(() => {
    setObjectMetaFilters.mockClear();
    appStoreState.selectedObjectType = null;
    appStoreState.objectMetaFilters = {};
    useGameSchemaMock.mockReturnValue({ data: undefined });
  });

  it('initializes basic state correctly', () => {
    const { result } = renderHook(() => useObjectListLogic(), { wrapper: createWrapper() });

    expect(result.current.state.isMobile).toBe(false);
    expect(result.current.state.objects).toEqual([]);
    expect(result.current.filters.activeFilters).toEqual({});
    expect(result.current.filters.sortBy).toBe('name');
  });

  it('updates filters properly', () => {
    const { result } = renderHook(() => useObjectListLogic(), { wrapper: createWrapper() });

    act(() => {
      result.current.filters.handleFilterChange('element', ['Fire']);
    });

    expect(setObjectMetaFilters).toHaveBeenCalledWith({ element: ['Fire'] });

    appStoreState.objectMetaFilters = { element: ['Fire'] };
    setObjectMetaFilters.mockClear();

    const { result: rerendered } = renderHook(() => useObjectListLogic(), {
      wrapper: createWrapper(),
    });
    act(() => {
      rerendered.current.filters.handleClearFilters();
    });

    expect(setObjectMetaFilters).toHaveBeenLastCalledWith({});
  });

  it('sanitizes invalid persisted filters without writing them back on mount', () => {
    appStoreState.selectedObjectType = 'Character';
    appStoreState.objectMetaFilters = { element: ['Pyro'], rarity: ['5'] };
    useGameSchemaMock.mockReturnValue({
      data: {
        categories: [
          {
            name: 'Character',
            filters: [{ key: 'element', label: 'Element', options: ['Pyro'] }],
          },
        ],
      },
    });

    const { result } = renderHook(() => useObjectListLogic(), { wrapper: createWrapper() });

    expect(result.current.filters.activeFilters).toEqual({ element: ['Pyro'] });
    expect(setObjectMetaFilters).not.toHaveBeenCalled();
  });
});
