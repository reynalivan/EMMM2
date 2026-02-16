/**
 * Tests for ResolutionModal component.
 * Covers: TC-9.5-03 (User confirmation before destructive operations)
 * Tests modal rendering, action summary, and confirmation flow.
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '../../../../test-utils';
import ResolutionModal from '../ResolutionModal';
import type { DupScanGroup } from '../../../../types/dedup';

// Mock HTMLDialogElement methods
beforeEach(() => {
  if (!HTMLDialogElement.prototype.showModal) {
    HTMLDialogElement.prototype.showModal = vi.fn(function (this: HTMLDialogElement) {
      this.style.display = 'block';
    });
  }
  if (!HTMLDialogElement.prototype.close) {
    HTMLDialogElement.prototype.close = vi.fn(function (this: HTMLDialogElement) {
      this.style.display = 'none';
    });
  }
});

describe('ResolutionModal', () => {
  const mockGroup: DupScanGroup = {
    groupId: 'group-1',
    confidenceScore: 95,
    matchReason: 'Hash match',
    signals: [],
    members: [
      {
        folderPath: '/path/mod-a',
        displayName: 'Mod A - Keep',
        totalSizeBytes: 2048,
        fileCount: 10,
        confidenceScore: 95,
        signals: [],
      },
      {
        folderPath: '/path/mod-b',
        displayName: 'Mod B - Delete',
        totalSizeBytes: 2048,
        fileCount: 10,
        confidenceScore: 95,
        signals: [],
      },
    ],
  };

  const defaultProps = {
    isOpen: false,
    selections: new Map<string, 'KeepA' | 'KeepB' | 'Ignore'>(),
    groups: [mockGroup],
    onConfirm: vi.fn(),
    onCancel: vi.fn(),
    isPending: false,
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Modal visibility', () => {
    it('does not render when isOpen is false', () => {
      render(<ResolutionModal {...defaultProps} isOpen={false} />);

      // Modal should exist but be in closed state
      const modal = screen.queryByRole('dialog');
      if (modal) {
        // If found, it should not be visible
        expect(modal).not.toHaveStyle('display: block');
      }
    });

    it('renders when isOpen is true', async () => {
      const selections = new Map([['group-1', 'KeepA' as const]]);

      render(<ResolutionModal {...defaultProps} isOpen={true} selections={selections} />);

      await waitFor(() => {
        expect(screen.getByText(/Confirm Resolution/)).toBeInTheDocument();
      });
    });
  });

  describe('Modal content', () => {
    it('displays header with warning icon', async () => {
      const selections = new Map([['group-1', 'KeepA' as const]]);

      render(<ResolutionModal {...defaultProps} isOpen={true} selections={selections} />);

      await waitFor(() => {
        expect(screen.getByText(/Confirm Resolution/)).toBeInTheDocument();
        expect(screen.getByText(/Review the actions/)).toBeInTheDocument();
      });
    });

    it('displays action summary with stats', async () => {
      const selections = new Map([['group-1', 'KeepA' as const]]);

      render(<ResolutionModal {...defaultProps} isOpen={true} selections={selections} />);

      await waitFor(() => {
        expect(screen.getByText(/Deletions/)).toBeInTheDocument();
        expect(screen.getByText(/Ignored/)).toBeInTheDocument();
        expect(screen.getByText(/Total/)).toBeInTheDocument();
      });
    });

    it('shows deletion count correctly', async () => {
      const selections = new Map<string, 'KeepA' | 'KeepB' | 'Ignore'>([['group-1', 'KeepA']]);

      render(<ResolutionModal {...defaultProps} isOpen={true} selections={selections} />);

      await waitFor(() => {
        // One deletion (Keep A = delete B)
        const stats = screen.getAllByText('1');
        expect(stats.length).toBeGreaterThan(0);
      });
    });

    it('shows ignore count correctly', async () => {
      const selections = new Map<string, 'KeepA' | 'KeepB' | 'Ignore'>([['group-1', 'Ignore']]);

      render(<ResolutionModal {...defaultProps} isOpen={true} selections={selections} />);

      await waitFor(() => {
        const deleteStats = screen.getAllByText('0');
        expect(deleteStats.length).toBeGreaterThan(0);
      });
    });
  });

  describe('Action breakdown', () => {
    it('displays detailed breakdown of KeepA action', async () => {
      const selections = new Map([['group-1', 'KeepA' as const]]);

      render(<ResolutionModal {...defaultProps} isOpen={true} selections={selections} />);

      await waitFor(() => {
        expect(screen.getByText(/Delete:/)).toBeInTheDocument();
        expect(screen.getByText('Mod B - Delete')).toBeInTheDocument();
        expect(screen.getByText(/Keep:/)).toBeInTheDocument();
        expect(screen.getByText(/Mod A - Keep/)).toBeInTheDocument();
      });
    });

    it('displays detailed breakdown of KeepB action', async () => {
      const selections = new Map([['group-1', 'KeepB' as const]]);

      render(<ResolutionModal {...defaultProps} isOpen={true} selections={selections} />);

      await waitFor(() => {
        expect(screen.getByText('Mod A - Keep')).toBeInTheDocument();
      });
    });

    it('displays detailed breakdown of Ignore action', async () => {
      const selections = new Map([['group-1', 'Ignore' as const]]);

      render(<ResolutionModal {...defaultProps} isOpen={true} selections={selections} />);

      await waitFor(() => {
        expect(screen.getByText(/Ignore:/)).toBeInTheDocument();
        expect(screen.getByText(/Mod A - Keep ↔ Mod B - Delete/)).toBeInTheDocument();
      });
    });

    it('lists all selected actions in breakdown', async () => {
      const multiGroup: DupScanGroup = {
        ...mockGroup,
        groupId: 'group-2',
        members: [
          {
            folderPath: '/path/mod-c',
            displayName: 'Mod C',
            totalSizeBytes: 2048,
            fileCount: 10,
            confidenceScore: 90,
            signals: [],
          },
          {
            folderPath: '/path/mod-d',
            displayName: 'Mod D',
            totalSizeBytes: 2048,
            fileCount: 10,
            confidenceScore: 90,
            signals: [],
          },
        ],
      };

      const selections = new Map<string, 'KeepA' | 'KeepB' | 'Ignore'>([
        ['group-1', 'KeepA'],
        ['group-2', 'KeepB'],
      ]);

      render(
        <ResolutionModal
          {...defaultProps}
          isOpen={true}
          selections={selections}
          groups={[mockGroup, multiGroup]}
        />,
      );

      await waitFor(() => {
        expect(screen.getByText('Mod B - Delete')).toBeInTheDocument();
        expect(screen.getByText(/Mod D/)).toBeInTheDocument();
      });
    });
  });

  describe('User interactions', () => {
    it('calls onCancel when Cancel button is clicked', async () => {
      const onCancel = vi.fn();
      const selections = new Map([['group-1', 'KeepA' as const]]);

      render(
        <ResolutionModal
          {...defaultProps}
          isOpen={true}
          selections={selections}
          onCancel={onCancel}
        />,
      );

      const cancelButton = await screen.findByRole('button', { name: /Cancel/i });
      fireEvent.click(cancelButton);

      expect(onCancel).toHaveBeenCalledOnce();
    });

    it('calls onConfirm when Confirm button is clicked', async () => {
      const onConfirm = vi.fn();
      const selections = new Map([['group-1', 'KeepA' as const]]);

      render(
        <ResolutionModal
          {...defaultProps}
          isOpen={true}
          selections={selections}
          onConfirm={onConfirm}
        />,
      );

      const confirmButton = await screen.findByRole('button', { name: /Confirm/ });
      fireEvent.click(confirmButton);

      expect(onConfirm).toHaveBeenCalledOnce();
    });

    it('confirm button shows action count', async () => {
      const selections = new Map<string, 'KeepA' | 'KeepB' | 'Ignore'>([['group-1', 'KeepA']]);

      render(<ResolutionModal {...defaultProps} isOpen={true} selections={selections} />);

      await waitFor(() => {
        const confirmButton = screen.getByRole('button', { name: /Confirm/ });
        expect(confirmButton).toHaveTextContent('Confirm (1)');
      });
    });
  });

  describe('Pending state', () => {
    it('disables buttons when isPending is true', async () => {
      const selections = new Map([['group-1', 'KeepA' as const]]);

      render(
        <ResolutionModal
          {...defaultProps}
          isOpen={true}
          selections={selections}
          isPending={true}
        />,
      );

      await waitFor(() => {
        const buttons = screen.getAllByRole('button');
        buttons.forEach((btn) => {
          if (btn.textContent?.includes('Cancel') || btn.textContent?.includes('Confirm')) {
            expect(btn).toBeDisabled();
          }
        });
      });
    });

    it('shows progress indicator when isPending is true', async () => {
      const selections = new Map([['group-1', 'KeepA' as const]]);

      render(
        <ResolutionModal
          {...defaultProps}
          isOpen={true}
          selections={selections}
          isPending={true}
        />,
      );

      await waitFor(() => {
        expect(screen.getByText(/Resolving duplicates/)).toBeInTheDocument();
      });
    });

    it('shows "Applying..." text on confirm button when pending', async () => {
      const selections = new Map([['group-1', 'KeepA' as const]]);

      render(
        <ResolutionModal
          {...defaultProps}
          isOpen={true}
          selections={selections}
          isPending={true}
        />,
      );

      await waitFor(() => {
        expect(screen.getByText(/Applying.../)).toBeInTheDocument();
      });
    });

    it('does not disable cancel button when not pending', async () => {
      const selections = new Map([['group-1', 'KeepA' as const]]);

      render(
        <ResolutionModal
          {...defaultProps}
          isOpen={true}
          selections={selections}
          isPending={false}
        />,
      );

      await waitFor(() => {
        const cancelButton = screen.getByRole('button', { name: /Cancel/i });
        expect(cancelButton).not.toBeDisabled();
      });
    });
  });

  describe('Accessibility', () => {
    it('has proper ARIA attributes', async () => {
      const selections = new Map([['group-1', 'KeepA' as const]]);

      render(<ResolutionModal {...defaultProps} isOpen={true} selections={selections} />);

      await waitFor(() => {
        const modal = screen.getByRole('dialog');
        expect(modal).toHaveAttribute('aria-modal', 'true');
        expect(modal).toHaveAttribute('aria-labelledby');
        expect(modal).toHaveAttribute('aria-describedby');
      });
    });

    it('breakdown list is accessible', async () => {
      const selections = new Map([['group-1', 'KeepA' as const]]);

      render(<ResolutionModal {...defaultProps} isOpen={true} selections={selections} />);

      await waitFor(() => {
        const list = screen.getByRole('list', { name: /Resolution action breakdown/i });
        expect(list).toBeInTheDocument();
      });
    });

    it('buttons have descriptive aria-labels', async () => {
      const selections = new Map([['group-1', 'KeepA' as const]]);

      render(<ResolutionModal {...defaultProps} isOpen={true} selections={selections} />);

      await waitFor(() => {
        const confirmButton = screen.getByRole('button', {
          name: /Confirm and apply 1 resolution actions/i,
        });
        expect(confirmButton).toBeInTheDocument();
      });
    });
  });

  describe('Edge cases', () => {
    it('handles multiple deletions correctly', async () => {
      const multiGroup: DupScanGroup = {
        ...mockGroup,
        groupId: 'group-2',
      };

      const selections = new Map<string, 'KeepA' | 'KeepB' | 'Ignore'>([
        ['group-1', 'KeepA'],
        ['group-2', 'KeepB'],
      ]);

      render(
        <ResolutionModal
          {...defaultProps}
          isOpen={true}
          selections={selections}
          groups={[mockGroup, multiGroup]}
        />,
      );

      await waitFor(() => {
        // Should show 2 deletions total
        const twos = screen.getAllByText('2');
        expect(twos.length).toBeGreaterThan(0);
      });
    });

    it('handles mixed actions (deletions and ignores)', async () => {
      const multiGroup: DupScanGroup = {
        ...mockGroup,
        groupId: 'group-2',
      };

      const selections = new Map<string, 'KeepA' | 'KeepB' | 'Ignore'>([
        ['group-1', 'KeepA'],
        ['group-2', 'Ignore'],
      ]);

      render(
        <ResolutionModal
          {...defaultProps}
          isOpen={true}
          selections={selections}
          groups={[mockGroup, multiGroup]}
        />,
      );

      await waitFor(() => {
        expect(screen.getByText(/Deletions/)).toBeInTheDocument();
        expect(screen.getByText(/Ignored/)).toBeInTheDocument();
      });
    });
  });
});
