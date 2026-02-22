import { render, screen } from '@testing-library/react';
import FolderGrid from './FolderGrid';
import { vi, describe, it, expect, beforeEach } from 'vitest';
import { createWrapper } from '../../test-utils';
import { ModFolder } from '../../types/mod';

// Mock the hook!
const mockUseFolderGrid = vi.fn();
vi.mock('./hooks/useFolderGrid', () => ({
  useFolderGrid: () => mockUseFolderGrid(),
}));

// Mock subcomponents
vi.mock('./FolderCard', () => ({
  default: ({ folder }: { folder: ModFolder }) => (
    <div data-testid="folder-card">{folder.name}</div>
  ),
}));

vi.mock('./FolderListRow', () => ({
  default: ({ item }: { item: ModFolder }) => <div data-testid="folder-row">{item.name}</div>,
}));

vi.mock('./Breadcrumbs', () => ({
  default: () => <div>Breadcrumbs</div>,
}));

vi.mock('./DragOverlay', () => ({
  default: () => <div>DragOverlay</div>,
}));

vi.mock('../../components/ui/ConfirmDialog', () => ({
  default: ({ open }: { open: boolean }) => (open ? <div>ConfirmDialog</div> : null),
}));

vi.mock('./BulkTagModal', () => ({
  BulkTagModal: ({ isOpen }: { isOpen: boolean }) => (isOpen ? <div>BulkTagModal</div> : null),
}));

const defaultHookReturn = {
  // Data & State
  sortedFolders: [],
  isLoading: false,
  isError: false,
  error: null,
  isGridView: true,
  isMobile: false,
  currentPath: [],
  explorerSearchQuery: '',
  sortOrder: 'asc',
  sortLabel: 'Name',
  viewMode: 'grid',

  // Virtualization
  parentRef: { current: null },
  rowVirtualizer: {
    getTotalSize: () => 1000,
    getVirtualItems: () => [],
  },
  columnCount: 4,
  cardWidth: 200,

  // Handlers mapped to vi.fn()
  handleNavigate: vi.fn(),
  handleBreadcrumbClick: vi.fn(),
  handleGoHome: vi.fn(),
  setMobilePane: vi.fn(),
  setViewMode: vi.fn(),
  setExplorerSearch: vi.fn(),
  handleSortToggle: vi.fn(),
  handleKeyDown: vi.fn(),
  focusedId: null,
  gridSelection: new Set(),
  toggleGridSelection: vi.fn(),
  clearGridSelection: vi.fn(),
  handleToggleEnabled: vi.fn(),
  handleToggleFavorite: vi.fn(),
  renamingId: null,
  handleRenameRequest: vi.fn(),
  handleRenameSubmit: vi.fn(),
  handleRenameCancel: vi.fn(),
  deleteConfirm: { open: false, folder: null },
  setDeleteConfirm: vi.fn(),
  handleDeleteRequest: vi.fn(),
  handleDeleteConfirm: vi.fn(),
  bulkTagOpen: false,
  setBulkTagOpen: vi.fn(),
  bulkDeleteConfirm: false,
  setBulkDeleteConfirm: vi.fn(),
  handleBulkToggle: vi.fn(),
  handleBulkTagRequest: vi.fn(),
  handleBulkDeleteRequest: vi.fn(),
  handleBulkDeleteConfirm: vi.fn(),
  isDragging: false,

  // Duplicate Warning
  duplicateWarning: { open: false, folder: null, duplicates: [] },
  handleDuplicateForceEnable: vi.fn(),
  handleDuplicateEnableOnly: vi.fn(),
  handleDuplicateCancel: vi.fn(),

  // Enable Only This
  handleEnableOnlyThis: vi.fn(),

  // Refresh
  handleRefresh: vi.fn(),
};

describe('FolderGrid', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockUseFolderGrid.mockReturnValue(defaultHookReturn);
  });

  it('renders empty state when no sortedFolders', () => {
    mockUseFolderGrid.mockReturnValue({
      ...defaultHookReturn,
      sortedFolders: [],
    });
    render(<FolderGrid />, { wrapper: createWrapper });
    expect(screen.getByText(/No mod folders found/i)).toBeInTheDocument();
  });

  it('renders loading state', () => {
    mockUseFolderGrid.mockReturnValue({
      ...defaultHookReturn,
      isLoading: true,
    });
    render(<FolderGrid />, { wrapper: createWrapper });
    // Assuming Loader2 renders something or we can find by class?
    // Or just check that empty state is NOT there
    expect(screen.queryByText(/No mod folders found/i)).not.toBeInTheDocument();
  });

  it('renders items', () => {
    const mockData: ModFolder[] = [
      {
        node_type: 'ContainerFolder',
        classification_reasons: [],
        name: 'Mod A',
        path: '/mods/Mod A',
        is_enabled: true,
        is_directory: true,
        folder_name: 'Mod A',
        thumbnail_path: null,
        modified_at: 0,
        size_bytes: 0,
        has_info_json: false,
        is_favorite: false,
        is_misplaced: false,
        is_safe: true,
        metadata: null,
        category: null,
      },
    ];

    mockUseFolderGrid.mockReturnValue({
      ...defaultHookReturn,
      sortedFolders: mockData,
      rowVirtualizer: {
        getTotalSize: () => 200,
        getVirtualItems: () => [{ index: 0, start: 0, size: 200, key: '0' }],
      },
      columnCount: 1, // list-like for verify
    });

    render(<FolderGrid />, { wrapper: createWrapper });
    expect(screen.getByText('Mod A')).toBeInTheDocument();
  });
});
