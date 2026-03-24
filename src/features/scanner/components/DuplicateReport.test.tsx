/**
 * Tests for DuplicateReport component.
 * Covers: TC-9.5-01, TC-9.5-02, TC-9.5-03
 * Tests main container logic, state management, user interactions.
 */

/* eslint-disable @typescript-eslint/no-explicit-any */
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '../../../testing/test-utils';
import DuplicateReport from './DuplicateReport';
import * as hooks from '../hooks/useDedup';
import type { DupScanReport, DupScanGroup } from '../../../types/scanner';

// Mock the hooks
vi.mock('../hooks/useDedup', () => ({
  useDedupReport: vi.fn(),
  useResolveDuplicates: vi.fn(),
}));

// Mock toast
vi.mock('../../../stores/useToastStore', () => ({
  toast: {
    warning: vi.fn(),
    error: vi.fn(),
    success: vi.fn(),
    info: vi.fn(),
  },
}));

// Mock child components
vi.mock('./DuplicateTable', () => ({
  default: ({
    groups,
    onSelectionChange,
    disabled,
  }: {
    groups: DupScanGroup[];
    onSelectionChange: (groupId: string, action: string) => void;
    disabled: boolean;
  }) => (
    <div data-testid="duplicate-table">
      {groups.map((g) => (
        <div key={g.groupId} data-testid={`group-${g.groupId}`}>
          <button
            data-testid={`keep-a-${g.groupId}`}
            onClick={() => onSelectionChange(g.groupId, 'KeepA')}
            disabled={disabled}
          >
            Keep A
          </button>
          <button
            data-testid={`keep-b-${g.groupId}`}
            onClick={() => onSelectionChange(g.groupId, 'KeepB')}
            disabled={disabled}
          >
            Keep B
          </button>
        </div>
      ))}
    </div>
  ),
}));

vi.mock('./ResolutionModal', () => ({
  default: ({ isOpen, onConfirm }: { isOpen: boolean; onConfirm: () => void }) => (
    <div data-testid="resolution-modal" style={{ display: isOpen ? 'block' : 'none' }}>
      <button data-testid="confirm-button" onClick={onConfirm}>
        Confirm
      </button>
    </div>
  ),
}));

const mockGroup: DupScanGroup = {
  groupId: 'group-1',
  confidenceScore: 95,
  matchReason: 'Perfect hash match',
  signals: [{ key: 'hash', detail: 'BLAKE3 collision', score: 100 }],
  isUnsafe: false,
  members: [
    {
      folderPath: '/path/mod-a',
      displayName: 'Mod A - Original',
      totalSizeBytes: 1024,
      fileCount: 5,
      confidenceScore: 95,
      signals: [],
      modId: null,
      isSafe: false,
    },
    {
      folderPath: '/path/mod-b',
      displayName: 'Mod B',
      totalSizeBytes: 1024,
      fileCount: 5,
      confidenceScore: 95,
      signals: [],
      modId: null,
      isSafe: false,
    },
  ],
};

const mockReport: DupScanReport = {
  scanId: 'scan-1',
  gameId: 'genshin',
  rootPath: '/path/to/mods',
  totalGroups: 1,
  totalMembers: 2,
  groups: [mockGroup],
};

describe('DuplicateReport', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Loading state', () => {
    it('displays loader while fetching report', () => {
      vi.mocked(hooks.useDedupReport).mockReturnValue({
        isLoading: true,
        isSuccess: false,
        isError: false,
        error: null,
        data: undefined,
      } as any);

      vi.mocked(hooks.useResolveDuplicates).mockReturnValue({
        isPending: false,
        mutate: vi.fn(),
      } as any);

      render(<DuplicateReport />);

      expect(screen.getByText(/loading duplicate report/i)).toBeInTheDocument();
    });
  });

  describe('Error state', () => {
    it('displays error alert on fetch failure', () => {
      const error = new Error('Failed to load');

      vi.mocked(hooks.useDedupReport).mockReturnValue({
        isLoading: false,
        isSuccess: false,
        isError: true,
        error,
        data: undefined,
      } as any);

      vi.mocked(hooks.useResolveDuplicates).mockReturnValue({
        isPending: false,
        mutate: vi.fn(),
      } as any);

      render(<DuplicateReport />);

      expect(screen.getByText(/failed to load duplicate report/i)).toBeInTheDocument();
    });
  });

  describe('Empty state', () => {
    it('displays message when no report exists', () => {
      vi.mocked(hooks.useDedupReport).mockReturnValue({
        isLoading: false,
        isSuccess: true,
        isError: false,
        error: null,
        data: null,
      } as any);

      vi.mocked(hooks.useResolveDuplicates).mockReturnValue({
        isPending: false,
        mutate: vi.fn(),
      } as any);

      render(<DuplicateReport />);

      expect(screen.getByText(/no scan results available/i)).toBeInTheDocument();
    });

    it('displays success message when no duplicates found', () => {
      const emptyReport: DupScanReport = {
        ...mockReport,
        totalGroups: 0,
        totalMembers: 0,
        groups: [],
      };

      vi.mocked(hooks.useDedupReport).mockReturnValue({
        isLoading: false,
        isSuccess: true,
        isError: false,
        error: null,
        data: emptyReport,
      } as any);

      vi.mocked(hooks.useResolveDuplicates).mockReturnValue({
        isPending: false,
        mutate: vi.fn(),
      } as any);

      render(<DuplicateReport />);

      expect(screen.getByText(/no duplicates found/i)).toBeInTheDocument();
    });
  });

  describe('Main content rendering', () => {
    it('displays report with duplicate groups', () => {
      vi.mocked(hooks.useDedupReport).mockReturnValue({
        isLoading: false,
        isSuccess: true,
        isError: false,
        error: null,
        data: mockReport,
      } as any);

      vi.mocked(hooks.useResolveDuplicates).mockReturnValue({
        isPending: false,
        mutate: vi.fn(),
      } as any);

      render(<DuplicateReport />);

      expect(screen.getByText(/Duplicate Scan Results/)).toBeInTheDocument();
      expect(screen.getByText(/Found 1 duplicate group/)).toBeInTheDocument();
    });

    it('renders duplicate table component', () => {
      vi.mocked(hooks.useDedupReport).mockReturnValue({
        isLoading: false,
        isSuccess: true,
        isError: false,
        error: null,
        data: mockReport,
      } as any);

      vi.mocked(hooks.useResolveDuplicates).mockReturnValue({
        isPending: false,
        mutate: vi.fn(),
      } as any);

      render(<DuplicateReport />);

      expect(screen.getByTestId('duplicate-table')).toBeInTheDocument();
    });
  });

  describe('User interactions', () => {
    beforeEach(() => {
      vi.mocked(hooks.useDedupReport).mockReturnValue({
        isLoading: false,
        isSuccess: true,
        isError: false,
        error: null,
        data: mockReport,
      } as any);

      vi.mocked(hooks.useResolveDuplicates).mockReturnValue({
        isPending: false,
        mutate: vi.fn(),
      } as any);
    });

    it('filters groups based on confidence score when activeFilter prop changes', () => {
      // High filter (score >= 100)
      const { rerender } = render(<DuplicateReport activeFilter="high" />);
      // Both mock groups are < 100
      expect(screen.getByText(/No duplicates found/i)).toBeInTheDocument();

      // Medium filter (score >= 70)
      rerender(<DuplicateReport activeFilter="medium" />);
      expect(screen.getByText('Mod A - Original')).toBeInTheDocument();
      expect(screen.getByText('Mod C')).toBeInTheDocument();

      // Low filter (score < 70)
      rerender(<DuplicateReport activeFilter="low" />);
      expect(screen.queryByText('Mod A - Original')).not.toBeInTheDocument();
    });

    it('updates selection when child table triggers onSelectionChange', async () => {
      render(<DuplicateReport activeFilter="all" />);

      // Find the first select (Resolution Action)
      const selects = screen.getAllByRole('combobox', { name: /Resolution Action/i });

      // Select "Keep A" for group 1
      fireEvent.change(selects[0], { target: { value: '/path/mod-a' } });

      // Click Apply button
      fireEvent.click(screen.getByRole('button', { name: /Apply 1 Action/i }));

      // Modal should show summary
      expect(screen.getByText(/Resolution Summary/i)).toBeInTheDocument();
      expect(screen.getByText('KEEP:')).toBeInTheDocument();
      expect(screen.getByText('Mod A - Original')).toBeInTheDocument();
    });

    it('disables Apply All button when no selections', () => {
      render(<DuplicateReport activeFilter="all" />);

      const applyButton = screen.getByRole('button', { name: /Apply 0 Actions/i });
      expect(applyButton).toBeDisabled();
    });

    it('enables Apply All button when selections exist', async () => {
      render(<DuplicateReport activeFilter="all" />);

      const selects = screen.getAllByRole('combobox', { name: /Resolution Action/i });
      fireEvent.change(selects[0], { target: { value: '/path/mod-a' } });

      expect(screen.getByText(/Apply 1 Action/i)).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /Apply 1 Action/i })).not.toBeDisabled();
    });

    it('shows warning toast when Apply All clicked with no selections', () => {
      vi.mocked(hooks.useDedupReport).mockReturnValue({
        isLoading: false,
        isSuccess: true,
        isError: false,
        error: null,
        data: mockReport,
      } as any);

      vi.mocked(hooks.useResolveDuplicates).mockReturnValue({
        isPending: false,
        mutate: vi.fn(),
      } as any);

      render(<DuplicateReport />);

      // The Apply button should be disabled when no selections exist
      const applyButton = screen.getByRole('button', { name: /Apply 0 Actions/i });
      expect(applyButton).toBeDisabled();
    });

    it('opens modal when Apply All clicked with selections', async () => {
      render(<DuplicateReport />);

      // Select an action
      const keepAButton = screen.getByTestId('keep-a-group-1');
      fireEvent.click(keepAButton);

      // Click Apply button
      const applyButton = await screen.findByRole('button', { name: /Apply 1 Action/i });
      fireEvent.click(applyButton);

      // Modal should be visible
      await waitFor(() => {
        const modal = screen.getByTestId('resolution-modal');
        expect(modal).toHaveStyle('display: block');
      });
    });

    it('triggers mutation on modal confirm', async () => {
      const mockMutate = vi.fn();

      vi.mocked(hooks.useResolveDuplicates).mockReturnValue({
        isPending: false,
        mutate: mockMutate,
      } as any);

      render(<DuplicateReport />);

      // Select action and apply
      const keepAButton = screen.getByTestId('keep-a-group-1');
      fireEvent.click(keepAButton);

      const applyButton = await screen.findByRole('button', { name: /Apply 1 Action/i });
      fireEvent.click(applyButton);

      // Confirm in modal
      const confirmButton = await screen.findByTestId('confirm-button');
      fireEvent.click(confirmButton);

      await waitFor(() => {
        expect(mockMutate).toHaveBeenCalled();
      });
    });
  });

  describe('Pending state', () => {
    it('disables interactions during resolution', () => {
      vi.mocked(hooks.useDedupReport).mockReturnValue({
        isLoading: false,
        isSuccess: true,
        isError: false,
        error: null,
        data: mockReport,
      } as any);

      vi.mocked(hooks.useResolveDuplicates).mockReturnValue({
        isPending: true,
        mutate: vi.fn(),
      } as any);

      render(<DuplicateReport />);

      const applyButton = screen.getByRole('button', { name: /Applying/i });
      expect(applyButton).toBeDisabled();
    });
  });
});
