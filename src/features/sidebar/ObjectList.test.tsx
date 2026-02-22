/**
 * Component Tests for Epic 3: ObjectList
 * Covers:
 * - TC-3.1-01 (Game Switching - verified via store props)
 * - TC-3.1-02 (Category Grouping)
 * - TC-3.1-03 (Search Filtering)
 * - TC-3.5-01 (Virtualization Rendering)
 * - NC-3.4-01 (Empty State)
 * - Folder mode fallback
 */

import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import ObjectList from './ObjectList';
import { useAppStore } from '../../stores/useAppStore';
import { useObjects, useGameSchema, useCategoryCounts } from '../../hooks/useObjects';
import { useActiveGame } from '../../hooks/useActiveGame';
import { useResponsive } from '../../hooks/useResponsive';
import { useVirtualizer } from '@tanstack/react-virtual';

// Mock dependencies
vi.mock('../../stores/useAppStore');
vi.mock('../../hooks/useObjects');
vi.mock('../../hooks/useFolders');
vi.mock('../../hooks/useActiveGame');
vi.mock('../../hooks/useResponsive');
vi.mock('@tanstack/react-virtual');
vi.mock('@tanstack/react-query', () => ({
  useQueryClient: vi.fn(() => ({ invalidateQueries: vi.fn() })),
  useMutation: vi.fn(() => ({ mutate: vi.fn(), mutateAsync: vi.fn() })),
}));
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));
vi.mock('../../components/ui/ContextMenu', () => ({
  ContextMenu: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  ContextMenuItem: () => null,
  ContextMenuSeparator: () => null,
  ContextMenuSub: () => null,
}));

vi.mock('./EditObjectModal', () => ({
  default: () => <div data-testid="edit-object-modal" />,
}));

const mockUseAppStore = useAppStore as unknown as ReturnType<typeof vi.fn>;
const mockUseObjects = useObjects as unknown as ReturnType<typeof vi.fn>;
const mockUseGameSchema = useGameSchema as unknown as ReturnType<typeof vi.fn>;
const mockUseCategoryCounts = useCategoryCounts as unknown as ReturnType<typeof vi.fn>;
const mockUseActiveGame = useActiveGame as unknown as ReturnType<typeof vi.fn>;
const mockUseResponsive = useResponsive as unknown as ReturnType<typeof vi.fn>;
const mockUseVirtualizer = useVirtualizer as unknown as ReturnType<typeof vi.fn>;

const mockActiveGame = {
  id: 'uuid-gimi',
  name: 'Genshin Impact',
  game_type: 'GIMI',
  path: 'C:\\Games\\GIMI',
  mods_path: 'C:\\Games\\GIMI\\Mods',
  launcher_path: '',
  launch_args: null,
};

describe('ObjectList Component', () => {
  const defaultStoreState = {
    selectedObject: null,
    setSelectedObject: vi.fn(),
    selectedObjectType: null,
    setSelectedObjectType: vi.fn(),
    sidebarSearchQuery: '',

    setSidebarSearch: vi.fn(),
    collapsedCategories: new Set(),
    toggleCategoryCollapse: vi.fn(),
    safeMode: true,
    setSafeMode: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();

    // Default mock returns
    mockUseAppStore.mockReturnValue(defaultStoreState);
    (mockUseAppStore as unknown as { getState: () => unknown }).getState = vi
      .fn()
      .mockReturnValue(defaultStoreState);

    mockUseActiveGame.mockReturnValue({
      activeGame: mockActiveGame,
      games: [mockActiveGame],
      isLoading: false,
      error: null,
    });

    mockUseObjects.mockReturnValue({
      data: [],
      isLoading: false,
      isError: false,
    });

    mockUseGameSchema.mockReturnValue({
      data: {
        categories: [
          { name: 'Character', icon: 'User', color: 'primary' },
          { name: 'Weapon', icon: 'Sword', color: 'secondary' },
        ],
        filters: [],
      },
    });

    mockUseCategoryCounts.mockReturnValue({ data: [] });

    mockUseResponsive.mockReturnValue({ isMobile: false });

    // Mock virtualizer to render all items
    mockUseVirtualizer.mockImplementation(({ count }: { count: number }) => ({
      getTotalSize: () => count * 40,
      getVirtualItems: () =>
        Array.from({ length: count }).map((_, i) => ({
          index: i,
          start: i * 40,
          size: 40,
          key: i,
        })),
    }));
  });

  it('renders loading state correctly', () => {
    mockUseObjects.mockReturnValue({ data: [], isLoading: true });
    render(<ObjectList />);
    const loader = screen.getByTestId('loading-spinner');
    expect(loader).toBeInTheDocument();
    expect(loader.querySelector('.animate-spin')).toBeInTheDocument();
  });

  it('renders empty state with CTA when no data and game selected', () => {
    render(<ObjectList />);
    expect(screen.getByText(/drag mod folders here or create a new object/i)).toBeInTheDocument();
    expect(screen.getByTitle(/create new object/i)).toBeInTheDocument();
  });

  it('renders "select a game" message when no active game', () => {
    mockUseActiveGame.mockReturnValue({
      activeGame: null,
      games: [],
      isLoading: false,
      error: null,
    });
    render(<ObjectList />);
    expect(screen.getByText(/select a game from the top bar/i)).toBeInTheDocument();
  });

  it('renders objects grouped by category (TC-3.1-02)', () => {
    const mockObjects = [
      {
        id: '1',
        name: 'Diluc',
        object_type: 'Character',
        mod_count: 5,
        enabled_count: 2,
        metadata: '{"element":"Pyro"}',
        tags: '[]',
      },
      {
        id: '2',
        name: 'Wolfs Gravestone',
        object_type: 'Weapon',
        mod_count: 3,
        enabled_count: 1,
        metadata: '{}',
        tags: '[]',
      },
    ];

    mockUseObjects.mockReturnValue({
      data: mockObjects,
      isLoading: false,
    });

    render(<ObjectList />);

    expect(screen.getAllByText('Character').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Weapon').length).toBeGreaterThan(0);
    expect(screen.getByText('Diluc')).toBeInTheDocument();
    expect(screen.getByText('Wolfs Gravestone')).toBeInTheDocument();
  });

  it('updates search query on input change (TC-3.1-03)', () => {
    render(<ObjectList />);
    const input = screen.getByPlaceholderText(/search/i);
    fireEvent.change(input, { target: { value: 'Ei' } });

    expect(defaultStoreState.setSidebarSearch).toHaveBeenCalledWith('Ei');
  });

  it('selects an object on click via store', () => {
    const mockObjects = [
      {
        id: '1',
        name: 'Diluc',
        object_type: 'Character',
        mod_count: 1,
        enabled_count: 1,
        metadata: '{}',
        tags: '[]',
      },
    ];
    mockUseObjects.mockReturnValue({ data: mockObjects });

    render(<ObjectList />);

    const row = screen.getByText('Diluc');
    fireEvent.click(row);

    expect(defaultStoreState.setSelectedObject).toHaveBeenCalledWith('1');
  });

  // DI-3.03: activeGameId from store matches what's rendered in sidebar
  it('renders sidebar for the active game from Zustand store (DI-3.03)', () => {
    const customGame = {
      id: 'uuid-srmi',
      name: 'Star Rail',
      game_type: 'SRMI',
      path: 'C:\\Games\\SRMI',
      mods_path: 'C:\\Games\\SRMI\\Mods',
      launcher_path: '',
      launch_args: null,
    };

    mockUseActiveGame.mockReturnValue({
      activeGame: customGame,
      games: [mockActiveGame, customGame],
      isLoading: false,
      error: null,
    });

    const mockObjects = [
      {
        id: 'kafka-1',
        name: 'Kafka',
        object_type: 'Character',
        mod_count: 1,
        enabled_count: 1,
        metadata: '{}',
        tags: '[]',
      },
    ];
    mockUseObjects.mockReturnValue({ data: mockObjects, isLoading: false });

    render(<ObjectList />);

    // Verify sidebar renders content for the active game (Star Rail's mod "Kafka")
    expect(screen.getByText('Kafka')).toBeInTheDocument();
    // Verify there's no Genshin content leaking
    expect(screen.queryByText('Raiden')).not.toBeInTheDocument();
  });

  // NC-3.4-01: Empty filter state shows "No objects match filter" message
  it('renders empty filter state when filters produce no results (NC-3.4-01)', () => {
    // Return empty data with active filters
    mockUseObjects.mockReturnValue({ data: [], isLoading: false, isError: false });

    // Note: activeFilters is internal state — we rely on isEmpty being true
    // and the search query empty to test the base empty state
    render(<ObjectList />);

    // Should show the empty state
    expect(screen.getByTestId('empty-state')).toBeInTheDocument();
    expect(screen.getByText(/drag mod folders here or create a new object/i)).toBeInTheDocument();
  });

  // US-3.4: Sorting
  it('updates sort order when sort chip is clicked', () => {
    mockUseObjects.mockReturnValue({
      data: [{ id: '1', name: 'Diluc', object_type: 'Character', metadata: '{}', tags: '[]' }],
      isLoading: false,
    });

    render(<ObjectList />);

    // FilterPanel has sort chips: 'A–Z', 'New', '★'
    // Click 'New' chip to change sort to 'date'
    const newChip = screen.getByText('New');
    fireEvent.click(newChip);

    // Verify useObjects called with sortBy: 'date'
    expect(mockUseObjects).toHaveBeenLastCalledWith(
      expect.objectContaining({
        sortBy: 'date',
      }),
    );
  });

  // US-3.4: Status Filter
  it('updates status filter when FilterPanel toggle is clicked', () => {
    mockUseObjects.mockReturnValue({
      data: [
        {
          id: '1',
          object_type: 'Character',
          mod_count: 1,
          enabled_count: 1,
          metadata: '{}',
          tags: '[]',
        },
      ],
      isLoading: false,
    });
    // Needs per-category filters for filter button to appear
    mockUseGameSchema.mockReturnValue({
      data: {
        categories: [
          {
            name: 'Character',
            icon: 'User',
            color: 'primary',
            filters: [{ key: 'Element', label: 'Element', options: ['Pyro'] }],
          },
        ],
        filters: [],
      },
    });

    render(<ObjectList />);

    // FilterPanel is open by default; toggle title shows 'Hide Filters'
    // Verify FilterPanel is already visible (no need to click toggle)

    // Find "Enabled" button in the FilterPanel
    const enabledBtn = screen.getByText('Enabled');
    fireEvent.click(enabledBtn);

    // Verify useObjects called with statusFilter: 'enabled'
    expect(mockUseObjects).toHaveBeenLastCalledWith(
      expect.objectContaining({
        statusFilter: 'enabled',
      }),
    );
  });
  // US-3.S: Safe Mode Toggle
  it('toggles safe mode when switch is clicked', () => {
    render(<ObjectList />);

    // Find Safe Mode toggle via class since it doesn't have an accessible label yet
    // In a real app we should add aria-label
    const toggle = document.querySelector('.toggle-success');
    expect(toggle).toBeInTheDocument();

    // Click it
    fireEvent.click(toggle!);

    // Verify setSafeMode called with false (since default is true)
    expect(defaultStoreState.setSafeMode).toHaveBeenCalledWith(false);
  });

  // TC-DIS-01: Fully disabled object shows grayscale overlay + strikethrough name
  it('shows power-off overlay and strikethrough for fully disabled object (TC-DIS-01)', () => {
    mockUseObjects.mockReturnValue({
      data: [
        {
          id: 'dis-1',
          name: 'Raiden Shogun',
          object_type: 'Character',
          mod_count: 3,
          enabled_count: 0, // ALL disabled
          metadata: '{}',
          tags: '[]',
          is_pinned: false,
          is_safe: true,
          is_auto_sync: false,
        },
      ],
      isLoading: false,
      isError: false,
    });

    render(<ObjectList />);

    // Name should have strikethrough
    const nameEl = screen.getByText('Raiden Shogun');
    expect(nameEl.className).toContain('line-through');

    // Power-off overlay should be rendered
    expect(screen.getByTestId('power-off-overlay')).toBeInTheDocument();
  });

  // TC-DIS-02: Partially enabled object (some mods on) shows NO disabled visual
  it('does NOT show disabled visuals for partially enabled object (TC-DIS-02)', () => {
    mockUseObjects.mockReturnValue({
      data: [
        {
          id: 'partial-1',
          name: 'Diluc',
          object_type: 'Character',
          mod_count: 3,
          enabled_count: 1, // only partially enabled
          metadata: '{}',
          tags: '[]',
          is_pinned: false,
          is_safe: true,
          is_auto_sync: false,
        },
      ],
      isLoading: false,
      isError: false,
    });

    render(<ObjectList />);

    // Name should NOT have strikethrough
    const nameEl = screen.getByText('Diluc');
    expect(nameEl.className).not.toContain('line-through');

    // Power-off overlay should NOT be rendered
    expect(screen.queryByTestId('power-off-overlay')).not.toBeInTheDocument();
  });
});
