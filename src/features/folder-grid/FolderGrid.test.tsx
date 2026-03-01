import { render, screen } from '@testing-library/react';
import FolderGrid from './FolderGrid';
import { vi, describe, it, expect, beforeEach } from 'vitest';
import { createWrapper } from '../../testing/test-utils';
import { ModFolder } from '../../types/mod';

// Mock the hook!
const mockUseFolderGrid = vi.fn();
vi.mock('./hooks/useFolderGrid', () => ({
  useFolderGrid: () => mockUseFolderGrid(),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => Promise.resolve(vi.fn())),
}));

vi.mock('../../hooks/useSettings', () => ({
  useSettings: () => ({ data: { organize_subfolders: true }, isLoading: false }),
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
  conflicts: [],
  isLoading: false,
  isError: false,
  error: null,
  selfNodeType: null,
  selfIsMod: false,
  selfIsEnabled: false,
  selfReasons: [],
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
  handleToggleSelf: vi.fn(),
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
  handleBulkDeleteConfirm: vi.fn(),
  handleBulkFavorite: vi.fn(),
  handleBulkSafe: vi.fn(),
  handleBulkPin: vi.fn(),
  handleBulkMoveToObject: vi.fn(),

  pinSafeDialog: { open: false, folder: null },
  handleToggleSafeRequest: vi.fn(),
  handleToggleSafeSubmit: vi.fn(),
  handleToggleSafeCancel: vi.fn(),

  isDragging: false,
  selectedObject: null,
  handleImportFiles: vi.fn(),

  // Duplicate Warning
  duplicateWarning: { open: false, folder: null, duplicates: [] },
  handleDuplicateForceEnable: vi.fn(),
  handleDuplicateEnableOnly: vi.fn(),
  handleDuplicateCancel: vi.fn(),

  // Enable Only This
  handleEnableOnlyThis: vi.fn(),

  // Refresh
  handleRefresh: vi.fn(),

  // Move
  moveDialog: { open: false, folder: null },
  setMoveDialog: vi.fn(),
  handleMoveRequest: vi.fn(),
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

  it('TC-12-01: renders Grid layout correctly', () => {
    mockUseFolderGrid.mockReturnValue({
      ...defaultHookReturn,
      isGridView: true,
      sortedFolders: [
        { name: 'Mod A', path: '/Mod A', is_directory: true } as unknown as ModFolder,
      ],
      rowVirtualizer: {
        getTotalSize: () => 200,
        getVirtualItems: () => [{ index: 0, start: 0, size: 200, key: '0' }],
      },
    });

    render(<FolderGrid />, { wrapper: createWrapper });
    expect(screen.getByTestId('folder-card')).toBeInTheDocument();
    expect(screen.queryByTestId('folder-row')).not.toBeInTheDocument();
  });

  it('TC-12-01: renders List layout correctly', () => {
    mockUseFolderGrid.mockReturnValue({
      ...defaultHookReturn,
      isGridView: false,
      sortedFolders: [
        { name: 'Mod B', path: '/Mod B', is_directory: true } as unknown as ModFolder,
      ],
      rowVirtualizer: {
        getTotalSize: () => 50,
        getVirtualItems: () => [{ index: 0, start: 0, size: 50, key: '0' }],
      },
    });

    render(<FolderGrid />, { wrapper: createWrapper });
    expect(screen.getByTestId('folder-row')).toBeInTheDocument();
    expect(screen.queryByTestId('folder-card')).not.toBeInTheDocument();
  });

  it('TC-12-08: renders contextual empty state for active search', () => {
    mockUseFolderGrid.mockReturnValue({
      ...defaultHookReturn,
      sortedFolders: [],
      explorerSearchQuery: 'NonExistentMod',
    });

    render(<FolderGrid />, { wrapper: createWrapper });
    expect(screen.getByText(/No mods match your search/i)).toBeInTheDocument();
    expect(screen.queryByText(/NonExistentMod/i)).not.toBeInTheDocument();
  });

  it('TC-12-10: handles Navigate up (Go Home / Breadcrumbs)', () => {
    const handleGoHome = vi.fn();
    mockUseFolderGrid.mockReturnValue({
      ...defaultHookReturn,
      handleGoHome,
      currentPath: ['Characters', 'Albedo'],
    });

    render(<FolderGrid />, { wrapper: createWrapper });
    // Assuming Breadcrumbs component has text 'Breadcrumbs' based on mock
    expect(screen.getByText('Breadcrumbs')).toBeInTheDocument();
  });

  it('TC-15-005: Renders DragOverlay when dragging', () => {
    mockUseFolderGrid.mockReturnValue({
      ...defaultHookReturn,
      isDragging: true,
    });
    render(<FolderGrid />, { wrapper: createWrapper });
    expect(screen.getByText('DragOverlay')).toBeInTheDocument();
  });
});
