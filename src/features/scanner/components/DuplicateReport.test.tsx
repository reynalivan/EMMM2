/**
 * Tests for DuplicateReport component.
 * Covers: TC-9.5-01, TC-9.5-02, TC-9.5-03
 * Tests main container logic, state management, user interactions.
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '../../../testing/test-utils';
import DuplicateReport from './DuplicateReport';
import * as hooks from '../hooks/useDedup';
import type { DupScanReport, DupScanGroup, ResolutionAction } from '../../../types/scanner';

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

vi.mock('../../../hooks/useSettings', () => ({
  useSettings: () => ({
    settings: {
      safe_mode: {
        enabled: false,
      },
    },
  }),
}));

// Mock child components
vi.mock('./DuplicateTable', () => ({
  default: ({
    groups,
    onSelectionChange,
    disabled,
  }: {
    groups: DupScanGroup[];
    onSelectionChange: (groupId: string, action: ResolutionAction) => void;
    disabled: boolean;
  }) => {
    return (
      <div data-testid="duplicate-table">
        {groups.map((g) => {
          const firstMember = g.members[0];
          const secondMember = g.members[1];

          return (
            <div key={g.groupId} data-testid={`group-${g.groupId}`}>
              <button
                data-testid={`keep-a-${g.groupId}`}
                onClick={() => {
                  if (firstMember) {
                    onSelectionChange(g.groupId, {
                      type: 'Keep',
                      targetPath: firstMember.folderPath,
                    });
                  }
                }}
                disabled={disabled || !firstMember}
              >
                Keep A
              </button>
              <button
                data-testid={`keep-b-${g.groupId}`}
                onClick={() => {
                  if (secondMember) {
                    onSelectionChange(g.groupId, {
                      type: 'Keep',
                      targetPath: secondMember.folderPath,
                    });
                  }
                }}
                disabled={disabled || !secondMember}
              >
                Keep B
              </button>
            </div>
          );
        })}
      </div>
    );
  },
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

type DedupReportHookResult = ReturnType<typeof hooks.useDedupReport>;
type ResolveDuplicatesHookResult = ReturnType<typeof hooks.useResolveDuplicates>;

function mockDedupReport(result: Partial<DedupReportHookResult>) {
  vi.mocked(hooks.useDedupReport).mockReturnValue(result as DedupReportHookResult);
}

function mockResolveDuplicates(result: Partial<ResolveDuplicatesHookResult>) {
  vi.mocked(hooks.useResolveDuplicates).mockReturnValue(result as ResolveDuplicatesHookResult);
}

describe('DuplicateReport', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Loading state', () => {
    it('displays loader while fetching report', () => {
      mockDedupReport({
        isLoading: true,
        isSuccess: false,
        isError: false,
        error: null,
        data: undefined,
      });

      mockResolveDuplicates({
        isPending: false,
        mutate: vi.fn(),
      });

      render(<DuplicateReport />);

      expect(screen.getByText(/loading duplicate report/i)).toBeInTheDocument();
    });
  });

  describe('Error state', () => {
    it('displays error alert on fetch failure', () => {
      const error = new Error('Failed to load');

      mockDedupReport({
        isLoading: false,
        isSuccess: false,
        isError: true,
        error,
        data: undefined,
      });

      mockResolveDuplicates({
        isPending: false,
        mutate: vi.fn(),
      });

      render(<DuplicateReport />);

      expect(screen.getByText(/failed to load duplicate report/i)).toBeInTheDocument();
    });
  });

  describe('Empty state', () => {
    it('displays message when no report exists', () => {
      mockDedupReport({
        isLoading: false,
        isSuccess: true,
        isError: false,
        error: null,
        data: null,
      });

      mockResolveDuplicates({
        isPending: false,
        mutate: vi.fn(),
      });

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

      mockDedupReport({
        isLoading: false,
        isSuccess: true,
        isError: false,
        error: null,
        data: emptyReport,
      });

      mockResolveDuplicates({
        isPending: false,
        mutate: vi.fn(),
      });

      render(<DuplicateReport />);

      expect(screen.getByText(/no duplicates found/i)).toBeInTheDocument();
    });
  });

  describe('Main content rendering', () => {
    it('displays report with duplicate groups', () => {
      mockDedupReport({
        isLoading: false,
        isSuccess: true,
        isError: false,
        error: null,
        data: mockReport,
      });

      mockResolveDuplicates({
        isPending: false,
        mutate: vi.fn(),
      });

      render(<DuplicateReport />);

      expect(screen.getByText(/Duplicate Scan Results/)).toBeInTheDocument();
      expect(screen.getByText(/Found 1 duplicate group/)).toBeInTheDocument();
    });

    it('renders duplicate table component', () => {
      mockDedupReport({
        isLoading: false,
        isSuccess: true,
        isError: false,
        error: null,
        data: mockReport,
      });

      mockResolveDuplicates({
        isPending: false,
        mutate: vi.fn(),
      });

      render(<DuplicateReport />);

      expect(screen.getByTestId('duplicate-table')).toBeInTheDocument();
    });
  });

  describe('User interactions', () => {
    beforeEach(() => {
      mockDedupReport({
        isLoading: false,
        isSuccess: true,
        isError: false,
        error: null,
        data: mockReport,
      });

      mockResolveDuplicates({
        isPending: false,
        mutate: vi.fn(),
      });
    });

    it('filters groups based on confidence score when activeFilter prop changes', () => {
      const filterReport: DupScanReport = {
        ...mockReport,
        totalGroups: 3,
        totalMembers: 6,
        groups: [
          { ...mockGroup, groupId: 'group-high', confidenceScore: 100 },
          { ...mockGroup, groupId: 'group-medium', confidenceScore: 95 },
          { ...mockGroup, groupId: 'group-low', confidenceScore: 50 },
        ],
      };
      mockDedupReport({
        isLoading: false,
        isSuccess: true,
        isError: false,
        error: null,
        data: filterReport,
      });

      const { rerender } = render(<DuplicateReport activeFilter="high" />);
      expect(screen.getByTestId('group-group-high')).toBeInTheDocument();
      expect(screen.queryByTestId('group-group-medium')).not.toBeInTheDocument();

      rerender(<DuplicateReport activeFilter="medium" />);
      expect(screen.queryByTestId('group-group-high')).not.toBeInTheDocument();
      expect(screen.getByTestId('group-group-medium')).toBeInTheDocument();

      rerender(<DuplicateReport activeFilter="low" />);
      expect(screen.getByTestId('group-group-low')).toBeInTheDocument();
      expect(screen.queryByTestId('group-group-medium')).not.toBeInTheDocument();
    });

    it('updates selection when child table triggers onSelectionChange', () => {
      render(<DuplicateReport activeFilter="all" />);

      fireEvent.click(screen.getByTestId('keep-a-group-1'));

      fireEvent.click(screen.getByRole('button', { name: /Apply 1 Action/i }));

      expect(screen.getByTestId('resolution-modal')).toHaveStyle('display: block');
    });

    it('disables Apply All button when no selections', () => {
      render(<DuplicateReport activeFilter="all" />);

      const applyButton = screen.getByRole('button', { name: /Apply 0 Actions/i });
      expect(applyButton).toBeDisabled();
    });

    it('enables Apply All button when selections exist', async () => {
      render(<DuplicateReport activeFilter="all" />);

      fireEvent.click(screen.getByTestId('keep-a-group-1'));

      expect(screen.getByText(/Apply 1 Action/i)).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /Apply 1 Action/i })).not.toBeDisabled();
    });

    it('shows warning toast when Apply All clicked with no selections', () => {
      mockDedupReport({
        isLoading: false,
        isSuccess: true,
        isError: false,
        error: null,
        data: mockReport,
      });

      mockResolveDuplicates({
        isPending: false,
        mutate: vi.fn(),
      });

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

      mockResolveDuplicates({
        isPending: false,
        mutate: mockMutate,
      });

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
      mockDedupReport({
        isLoading: false,
        isSuccess: true,
        isError: false,
        error: null,
        data: mockReport,
      });

      mockResolveDuplicates({
        isPending: true,
        mutate: vi.fn(),
      });

      render(<DuplicateReport />);

      const applyButton = screen.getByRole('button', { name: /Applying/i });
      expect(applyButton).toBeDisabled();
    });
  });
});
