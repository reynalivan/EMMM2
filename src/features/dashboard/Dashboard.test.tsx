/**
 * Tests for Dashboard component.
 * Covers: TC-33-001, TC-33-003, TC-33-006 (Stat Cards, Empty State, Quick Play)
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '../../testing/test-utils';
import Dashboard from './Dashboard';
import { useAppStore } from '../../stores/useAppStore';

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// Mock recharts to avoid canvas rendering issues in jsdom
vi.mock('recharts', () => ({
  PieChart: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="pie-chart">{children}</div>
  ),
  Pie: () => <div data-testid="pie" />,
  Cell: () => null,
  BarChart: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="bar-chart">{children}</div>
  ),
  Bar: () => <div data-testid="bar" />,
  XAxis: () => null,
  YAxis: () => null,
  Tooltip: () => null,
  ResponsiveContainer: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="responsive-container">{children}</div>
  ),
  Legend: () => null,
}));

// Mock hooks
vi.mock('./hooks/useDashboardStats', () => ({
  useDashboardStats: vi.fn(),
}));

vi.mock('./hooks/useActiveKeybindings', () => ({
  useActiveKeybindings: vi.fn(),
}));

vi.mock('../../hooks/useActiveGame', () => ({
  useActiveGame: () => ({
    activeGame: {
      id: 'g-1',
      name: 'Genshin Impact',
      game_type: 'GIMI',
      mod_path: 'E:/Mods',
      game_exe: 'E:/Game/Genshin.exe',
      loader_exe: null,
      launch_args: null,
    },
  }),
}));

import { useDashboardStats } from './hooks/useDashboardStats';
import { useActiveKeybindings } from './hooks/useActiveKeybindings';

const mockFullStats = {
  stats: {
    total_mods: 42,
    enabled_mods: 30,
    disabled_mods: 12,
    total_games: 2,
    total_size_bytes: 1073741824, // 1 GB
    total_collections: 3,
  },
  duplicate_waste_bytes: 0,
  category_distribution: [
    { category: 'Character', count: 25 },
    { category: 'Weapon', count: 10 },
    { category: 'UI', count: 7 },
  ],
  game_distribution: [
    { game_id: 'g-1', game_name: 'Genshin Impact', count: 30 },
    { game_id: 'g-2', game_name: 'HSR', count: 12 },
  ],
  recent_mods: [
    {
      id: 'mod-1',
      name: 'Hu Tao Quantum Mod',
      game_name: 'Genshin Impact',
      object_name: 'HuTao',
      indexed_at: new Date().toISOString(),
    },
  ],
};

describe('Dashboard - TC-33', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useAppStore.setState({ safeMode: false });

    vi.mocked(useActiveKeybindings).mockReturnValue({
      keybindings: [],
      isLoading: false,
      isError: false,
    });
  });

  describe('TC-33-001: Loading State', () => {
    it('shows skeleton UI while loading', () => {
      vi.mocked(useDashboardStats).mockReturnValue({
        data: undefined,
        isLoading: true,
        isError: false,
        error: null,
        refresh: vi.fn(),
      });

      render(<Dashboard />);

      // DashboardSkeleton renders animate-pulse
      expect(document.querySelector('.animate-pulse')).toBeInTheDocument();
    });
  });

  describe('TC-33-002: Error State', () => {
    it('shows error alert and retry button when data fails to load', () => {
      vi.mocked(useDashboardStats).mockReturnValue({
        data: undefined,
        isLoading: false,
        isError: true,
        error: new Error('Network error'),
        refresh: vi.fn(),
      });

      render(<Dashboard />);

      expect(screen.getByText(/failed to load dashboard data/i)).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /retry/i })).toBeInTheDocument();
    });

    it('calls refresh when Retry is clicked', () => {
      const mockRefresh = vi.fn();
      vi.mocked(useDashboardStats).mockReturnValue({
        data: undefined,
        isLoading: false,
        isError: true,
        error: new Error('Network error'),
        refresh: mockRefresh,
      });

      render(<Dashboard />);

      fireEvent.click(screen.getByRole('button', { name: /retry/i }));
      expect(mockRefresh).toHaveBeenCalledTimes(1);
    });
  });

  describe('TC-33-003: Empty State (No Games)', () => {
    it('shows welcome empty state when total_games is 0', () => {
      vi.mocked(useDashboardStats).mockReturnValue({
        data: {
          ...mockFullStats,
          stats: { ...mockFullStats.stats, total_games: 0 },
        },
        isLoading: false,
        isError: false,
        error: null,
        refresh: vi.fn(),
      });

      render(<Dashboard />);

      expect(screen.getByText(/Welcome to EMMM2/i)).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /Add Your First Game/i })).toBeInTheDocument();
    });
  });

  describe('TC-33-004: Stat Cards Rendering', () => {
    beforeEach(() => {
      vi.mocked(useDashboardStats).mockReturnValue({
        data: mockFullStats,
        isLoading: false,
        isError: false,
        error: null,
        refresh: vi.fn(),
      });
    });

    it('renders Total Mods stat tile correctly', () => {
      render(<Dashboard />);

      expect(screen.getByText('Total Mods')).toBeInTheDocument();
      expect(screen.getByText('42')).toBeInTheDocument();
      expect(screen.getByText(/30 enabled · 12 disabled/i)).toBeInTheDocument();
    });

    it('renders Games stat tile correctly', () => {
      render(<Dashboard />);

      expect(screen.getByText('Games')).toBeInTheDocument();
      expect(screen.getByText('2')).toBeInTheDocument();
    });

    it('renders Storage stat tile with formatted bytes', () => {
      render(<Dashboard />);

      expect(screen.getByText('Storage')).toBeInTheDocument();
      // 1 GB
      expect(screen.getByText('1.0 GB')).toBeInTheDocument();
    });

    it('renders Collections stat tile', () => {
      render(<Dashboard />);

      // Collections appears in both the quick-action bar and stat tile
      const collectionsElements = screen.getAllByText('Collections');
      expect(collectionsElements.length).toBeGreaterThanOrEqual(1);
      expect(screen.getByText('3')).toBeInTheDocument();
    });
  });

  describe('TC-33-005: Duplicate Waste Banner', () => {
    it('shows waste banner when duplicate_waste_bytes > 0', () => {
      vi.mocked(useDashboardStats).mockReturnValue({
        data: { ...mockFullStats, duplicate_waste_bytes: 524288000 }, // 500 MB
        isLoading: false,
        isError: false,
        error: null,
        refresh: vi.fn(),
      });

      render(<Dashboard />);

      expect(screen.getByText(/duplicate waste detected/i)).toBeInTheDocument();
      expect(screen.getByText(/500\.0 MB wasted/i)).toBeInTheDocument();
    });

    it('does not show waste alert when no duplicates', () => {
      vi.mocked(useDashboardStats).mockReturnValue({
        data: mockFullStats, // duplicate_waste_bytes: 0
        isLoading: false,
        isError: false,
        error: null,
        refresh: vi.fn(),
      });

      render(<Dashboard />);

      expect(screen.queryByText(/duplicate waste detected/i)).not.toBeInTheDocument();
    });
  });

  describe('TC-33-006: Quick Play', () => {
    it('shows active game name in quick play panel', () => {
      vi.mocked(useDashboardStats).mockReturnValue({
        data: mockFullStats,
        isLoading: false,
        isError: false,
        error: null,
        refresh: vi.fn(),
      });

      render(<Dashboard />);

      expect(screen.getByText('Genshin Impact')).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /launch/i })).toBeInTheDocument();
    });

    it('calls launch_game invoke on quick play button click', async () => {
      const { invoke } = await import('@tauri-apps/api/core');
      vi.mocked(invoke).mockResolvedValueOnce(undefined);

      vi.mocked(useDashboardStats).mockReturnValue({
        data: mockFullStats,
        isLoading: false,
        isError: false,
        error: null,
        refresh: vi.fn(),
      });

      render(<Dashboard />);

      fireEvent.click(screen.getByRole('button', { name: /launch/i }));

      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith('launch_game', { gameId: 'g-1' });
      });
    });
  });

  describe('TC-33-007: Active Keybindings Widget', () => {
    it('shows keybinding table when keybindings exist', () => {
      vi.mocked(useDashboardStats).mockReturnValue({
        data: mockFullStats,
        isLoading: false,
        isError: false,
        error: null,
        refresh: vi.fn(),
      });

      vi.mocked(useActiveKeybindings).mockReturnValue({
        keybindings: [
          {
            mod_name: 'Hu Tao Mod',
            section_name: '[Key1]',
            key: 'F1',
            back: 'F2',
          },
        ],
        isLoading: false,
        isError: false,
      });

      render(<Dashboard />);

      expect(screen.getByText('Active Key Mapping')).toBeInTheDocument();
      expect(screen.getByText('Hu Tao Mod')).toBeInTheDocument();
      expect(screen.getByText('F1')).toBeInTheDocument();
    });

    it('shows empty keybinding message when no keybindings found', () => {
      vi.mocked(useDashboardStats).mockReturnValue({
        data: mockFullStats,
        isLoading: false,
        isError: false,
        error: null,
        refresh: vi.fn(),
      });

      vi.mocked(useActiveKeybindings).mockReturnValue({
        keybindings: [],
        isLoading: false,
        isError: false,
      });

      render(<Dashboard />);

      expect(screen.getByText(/no keybindings found in enabled mods/i)).toBeInTheDocument();
    });
  });

  describe('TC-33-008: Recently Added', () => {
    it('shows recently added mods list', () => {
      vi.mocked(useDashboardStats).mockReturnValue({
        data: mockFullStats,
        isLoading: false,
        isError: false,
        error: null,
        refresh: vi.fn(),
      });

      render(<Dashboard />);

      expect(screen.getByText('Hu Tao Quantum Mod')).toBeInTheDocument();
    });

    it('shows empty message when no recent mods', () => {
      vi.mocked(useDashboardStats).mockReturnValue({
        data: { ...mockFullStats, recent_mods: [] },
        isLoading: false,
        isError: false,
        error: null,
        refresh: vi.fn(),
      });

      render(<Dashboard />);

      expect(screen.getByText(/no mods indexed yet/i)).toBeInTheDocument();
    });
  });

  describe('TC-33-009: Chart Rendering', () => {
    it('renders category distribution chart when data exists', () => {
      vi.mocked(useDashboardStats).mockReturnValue({
        data: mockFullStats,
        isLoading: false,
        isError: false,
        error: null,
        refresh: vi.fn(),
      });

      render(<Dashboard />);

      expect(screen.getByText('Category Distribution')).toBeInTheDocument();
      expect(screen.getByTestId('pie-chart')).toBeInTheDocument();
    });

    it('renders mods per game bar chart when data exists', () => {
      vi.mocked(useDashboardStats).mockReturnValue({
        data: mockFullStats,
        isLoading: false,
        isError: false,
        error: null,
        refresh: vi.fn(),
      });

      render(<Dashboard />);

      expect(screen.getByText('Mods per Game')).toBeInTheDocument();
      expect(screen.getByTestId('bar-chart')).toBeInTheDocument();
    });
  });
});
