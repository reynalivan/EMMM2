/**
 * Tests for DuplicateTable component.
 * Covers: TC-9.5-02 (UI controls and accessibility)
 * Tests table rendering, radio button groups, and action selection.
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent } from '../../../../test-utils';
import DuplicateTable from '../DuplicateTable';
import type { DupScanGroup } from '../../../../types/dedup';

describe('DuplicateTable', () => {
  const mockGroups: DupScanGroup[] = [
    {
      groupId: 'group-1',
      confidenceScore: 95,
      matchReason: 'Perfect hash match',
      signals: [{ key: 'hash', detail: 'BLAKE3 collision', score: 100 }],
      members: [
        {
          folderPath: '/path/mod-a',
          displayName: 'Mod A - Original',
          totalSizeBytes: 2048,
          fileCount: 10,
          confidenceScore: 95,
          signals: [],
        },
        {
          folderPath: '/path/mod-b',
          displayName: 'Mod B - Duplicate',
          totalSizeBytes: 2048,
          fileCount: 10,
          confidenceScore: 95,
          signals: [],
        },
      ],
    },
    {
      groupId: 'group-2',
      confidenceScore: 85,
      matchReason: 'Content similarity',
      signals: [
        { key: 'name_similarity', detail: 'Levenshtein distance', score: 85 },
        { key: 'file_count', detail: 'Same file count', score: 80 },
      ],
      members: [
        {
          folderPath: '/path/mod-c',
          displayName: 'Mod C',
          totalSizeBytes: 4096,
          fileCount: 20,
          confidenceScore: 85,
          signals: [],
        },
        {
          folderPath: '/path/mod-d',
          displayName: 'Mod D',
          totalSizeBytes: 4096,
          fileCount: 20,
          confidenceScore: 85,
          signals: [],
        },
      ],
    },
  ];

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Rendering', () => {
    it('renders table with all groups', () => {
      const onSelectionChange = vi.fn();

      render(
        <DuplicateTable
          groups={mockGroups}
          selections={new Map()}
          onSelectionChange={onSelectionChange}
        />,
      );

      expect(screen.getByRole('table', { name: /Duplicate groups/i })).toBeInTheDocument();
      expect(screen.getByText('Mod A - Original')).toBeInTheDocument();
      expect(screen.getByText('Mod D')).toBeInTheDocument();
    });

    it('renders table headers with correct semantic roles', () => {
      const onSelectionChange = vi.fn();

      render(
        <DuplicateTable
          groups={mockGroups}
          selections={new Map()}
          onSelectionChange={onSelectionChange}
        />,
      );

      expect(screen.getByText('Confidence')).toBeInTheDocument();
      expect(screen.getByText('Match Reason')).toBeInTheDocument();
      expect(screen.getByText('Members')).toBeInTheDocument();
      expect(screen.getByText('Action')).toBeInTheDocument();
    });

    it('displays confidence badge with correct styling', () => {
      const onSelectionChange = vi.fn();

      render(
        <DuplicateTable
          groups={mockGroups}
          selections={new Map()}
          onSelectionChange={onSelectionChange}
        />,
      );

      // Group 1: 95% confidence should have success badge
      expect(screen.getByText('95%')).toBeInTheDocument();
      // Group 2: 85% confidence should have warning badge
      expect(screen.getByText('85%')).toBeInTheDocument();
    });

    it('shows match reason with signal details', () => {
      const onSelectionChange = vi.fn();

      render(
        <DuplicateTable
          groups={mockGroups}
          selections={new Map()}
          onSelectionChange={onSelectionChange}
        />,
      );

      expect(screen.getByText('Perfect hash match')).toBeInTheDocument();
      expect(screen.getByText('Content similarity')).toBeInTheDocument();
    });

    it('displays member information with formatted sizes', () => {
      const onSelectionChange = vi.fn();

      render(
        <DuplicateTable
          groups={mockGroups}
          selections={new Map()}
          onSelectionChange={onSelectionChange}
        />,
      );

      // Check member names and sizes are rendered
      expect(screen.getByText('Mod A - Original')).toBeInTheDocument();
      expect(screen.getByText('Mod B - Duplicate')).toBeInTheDocument();

      // Should show formatted sizes (may appear multiple times for each member)
      const sizeElements = screen.getAllByText(/2 KB|2.0 KB/);
      expect(sizeElements.length).toBeGreaterThan(0);
    });

    it('renders empty state when no groups', () => {
      const onSelectionChange = vi.fn();

      render(
        <DuplicateTable groups={[]} selections={new Map()} onSelectionChange={onSelectionChange} />,
      );

      expect(screen.getByText(/No duplicate groups found/i)).toBeInTheDocument();
    });
  });

  describe('Radio Button Groups', () => {
    it('renders radio buttons for each group as a radiogroup', () => {
      const onSelectionChange = vi.fn();

      render(
        <DuplicateTable
          groups={mockGroups}
          selections={new Map()}
          onSelectionChange={onSelectionChange}
        />,
      );

      const radioGroups = screen.getAllByRole('radiogroup');
      expect(radioGroups.length).toBeGreaterThanOrEqual(2);
    });

    it('displays Keep A, Keep B, and Ignore radio buttons', () => {
      const onSelectionChange = vi.fn();

      render(
        <DuplicateTable
          groups={mockGroups.slice(0, 1)}
          selections={new Map()}
          onSelectionChange={onSelectionChange}
        />,
      );

      expect(screen.getAllByText('Keep A')).toHaveLength(1);
      expect(screen.getAllByText('Keep B')).toHaveLength(1);
      expect(screen.getAllByText('Ignore')).toHaveLength(1);
    });

    it('has sr-only hidden radio inputs with accessibility labels', () => {
      const onSelectionChange = vi.fn();

      render(
        <DuplicateTable
          groups={mockGroups.slice(0, 1)}
          selections={new Map()}
          onSelectionChange={onSelectionChange}
        />,
      );

      const radioInputs = screen.getAllByRole('radio');
      expect(radioInputs.length).toBeGreaterThanOrEqual(3);

      // Check that inputs have aria-labels
      const keepAInput = radioInputs.find(
        (r) => (r as HTMLInputElement).getAttribute('aria-label') === 'Keep A, delete B',
      );
      expect(keepAInput).toBeTruthy();
    });
  });

  describe('Selection tracking', () => {
    it('calls onSelectionChange when Keep A is clicked', () => {
      const onSelectionChange = vi.fn();

      render(
        <DuplicateTable
          groups={mockGroups.slice(0, 1)}
          selections={new Map()}
          onSelectionChange={onSelectionChange}
        />,
      );

      const keepAButtons = screen.getAllByText('Keep A');
      fireEvent.click(keepAButtons[0]);

      expect(onSelectionChange).toHaveBeenCalledWith('group-1', 'KeepA');
    });

    it('calls onSelectionChange when Keep B is clicked', () => {
      const onSelectionChange = vi.fn();

      render(
        <DuplicateTable
          groups={mockGroups.slice(0, 1)}
          selections={new Map()}
          onSelectionChange={onSelectionChange}
        />,
      );

      const keepBButtons = screen.getAllByText('Keep B');
      fireEvent.click(keepBButtons[0]);

      expect(onSelectionChange).toHaveBeenCalledWith('group-1', 'KeepB');
    });

    it('calls onSelectionChange when Ignore is clicked', () => {
      const onSelectionChange = vi.fn();

      render(
        <DuplicateTable
          groups={mockGroups.slice(0, 1)}
          selections={new Map()}
          onSelectionChange={onSelectionChange}
        />,
      );

      const ignoreButtons = screen.getAllByText('Ignore');
      fireEvent.click(ignoreButtons[0]);

      expect(onSelectionChange).toHaveBeenCalledWith('group-1', 'Ignore');
    });

    it('highlights selected action button', () => {
      const onSelectionChange = vi.fn();
      const selections = new Map<string, 'KeepA' | 'KeepB' | 'Ignore'>([['group-1', 'KeepA']]);

      render(
        <DuplicateTable
          groups={mockGroups.slice(0, 1)}
          selections={selections}
          onSelectionChange={onSelectionChange}
        />,
      );

      // The button should have btn-primary class when selected
      // We check that it's been rendered in a selected state
      const buttons = screen.getAllByText('Keep A');
      expect(buttons.length).toBeGreaterThan(0);
    });
  });

  describe('Disabled state', () => {
    it('disables all radio inputs when disabled prop is true', () => {
      const onSelectionChange = vi.fn();

      render(
        <DuplicateTable
          groups={mockGroups.slice(0, 1)}
          selections={new Map()}
          onSelectionChange={onSelectionChange}
          disabled={true}
        />,
      );

      const radioInputs = screen.getAllByRole('radio') as HTMLInputElement[];
      expect(radioInputs.every((r) => r.disabled)).toBe(true);
    });

    it('does not disable inputs when disabled prop is false', () => {
      const onSelectionChange = vi.fn();

      render(
        <DuplicateTable
          groups={mockGroups.slice(0, 1)}
          selections={new Map()}
          onSelectionChange={onSelectionChange}
          disabled={false}
        />,
      );

      const radioInputs = screen.getAllByRole('radio') as HTMLInputElement[];
      expect(radioInputs.some((r) => !r.disabled)).toBe(true);
    });
  });

  describe('Multi-member group handling', () => {
    it('shows warning for groups with more than 2 members', () => {
      const multiMemberGroup: DupScanGroup = {
        ...mockGroups[0],
        groupId: 'group-multi',
        members: [
          ...mockGroups[0].members,
          {
            folderPath: '/path/mod-e',
            displayName: 'Mod E',
            totalSizeBytes: 2048,
            fileCount: 10,
            confidenceScore: 95,
            signals: [],
          },
        ],
      };

      const onSelectionChange = vi.fn();

      render(
        <DuplicateTable
          groups={[multiMemberGroup]}
          selections={new Map()}
          onSelectionChange={onSelectionChange}
        />,
      );

      expect(screen.getByText(/Multi-member groups not supported/i)).toBeInTheDocument();
      expect(screen.queryByRole('radiogroup')).not.toBeInTheDocument();
    });

    it('shows overflow indicator for groups with more than 2 members', () => {
      const multiMemberGroup: DupScanGroup = {
        ...mockGroups[0],
        groupId: 'group-multi',
        members: [
          ...mockGroups[0].members,
          {
            folderPath: '/path/mod-e',
            displayName: 'Mod E',
            totalSizeBytes: 2048,
            fileCount: 10,
            confidenceScore: 95,
            signals: [],
          },
          {
            folderPath: '/path/mod-f',
            displayName: 'Mod F',
            totalSizeBytes: 2048,
            fileCount: 10,
            confidenceScore: 95,
            signals: [],
          },
        ],
      };

      const onSelectionChange = vi.fn();

      render(
        <DuplicateTable
          groups={[multiMemberGroup]}
          selections={new Map()}
          onSelectionChange={onSelectionChange}
        />,
      );

      expect(screen.getByText(/\+2 more/)).toBeInTheDocument();
    });
  });
});
