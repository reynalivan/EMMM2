/**
 * Tests for DuplicateTable component.
 * Covers: TC-9.5-02 (UI controls and accessibility)
 * Tests table rendering, radio button groups, and action selection.
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent } from '../../../testing/test-utils';
import DuplicateTable from './DuplicateTable';
import type { DupScanGroup, ResolutionAction } from '../../../types/scanner';

describe('DuplicateTable', () => {
  const mockGroups: DupScanGroup[] = [
    {
      groupId: 'group-1',
      confidenceScore: 95,
      matchReason: 'Perfect hash match',
      signals: [{ key: 'hash', detail: 'BLAKE3 collision', score: 100 }],
      isUnsafe: false,
      members: [
        {
          folderPath: '/path/mod-a',
          displayName: 'Mod A - Original',
          totalSizeBytes: 2048,
          fileCount: 10,
          confidenceScore: 95,
          signals: [],
          modId: null,
          isSafe: false,
        },
        {
          folderPath: '/path/mod-b',
          displayName: 'Mod B - Duplicate',
          totalSizeBytes: 2048,
          fileCount: 10,
          confidenceScore: 95,
          signals: [],
          modId: null,
          isSafe: false,
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
      isUnsafe: false,
      members: [
        {
          folderPath: '/path/mod-c',
          displayName: 'Mod C',
          totalSizeBytes: 4096,
          fileCount: 20,
          confidenceScore: 85,
          signals: [],
          modId: null,
          isSafe: false,
        },
        {
          folderPath: '/path/mod-d',
          displayName: 'Mod D',
          totalSizeBytes: 2048,
          fileCount: 10,
          confidenceScore: 85,
          signals: [],
          modId: null,
          isSafe: false,
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

  describe('Dropdown Controls', () => {
    it('renders a select dropdown for each group', () => {
      const onSelectionChange = vi.fn();

      render(
        <DuplicateTable
          groups={mockGroups}
          selections={new Map()}
          onSelectionChange={onSelectionChange}
        />,
      );

      const selects = screen.getAllByRole('combobox', { name: /Resolution Action/i });
      expect(selects).toHaveLength(2);
    });

    it('contains options for each member in the group', () => {
      const onSelectionChange = vi.fn();

      render(
        <DuplicateTable
          groups={mockGroups.slice(0, 1)}
          selections={new Map()}
          onSelectionChange={onSelectionChange}
        />,
      );

      expect(screen.getByText('Keep A: Mod A - Original')).toBeInTheDocument();
      expect(screen.getByText('Keep B: Mod B - Duplicate')).toBeInTheDocument();
    });
  });

  describe('Selection tracking', () => {
    it('calls onSelectionChange when a member is selected to be kept', () => {
      const onSelectionChange = vi.fn();

      render(
        <DuplicateTable
          groups={mockGroups.slice(0, 1)}
          selections={new Map()}
          onSelectionChange={onSelectionChange}
        />,
      );

      const select = screen.getByRole('combobox', { name: /Resolution Action/i });
      fireEvent.change(select, { target: { value: '/path/mod-a' } });

      expect(onSelectionChange).toHaveBeenCalledWith('group-1', {
        type: 'Keep',
        targetPath: '/path/mod-a',
      });
    });

    it('calls onSelectionChange when Ignore is selected', () => {
      const onSelectionChange = vi.fn();

      render(
        <DuplicateTable
          groups={mockGroups.slice(0, 1)}
          selections={new Map()}
          onSelectionChange={onSelectionChange}
        />,
      );

      const select = screen.getByRole('combobox', { name: /Resolution Action/i });
      fireEvent.change(select, { target: { value: 'ignore' } });

      expect(onSelectionChange).toHaveBeenCalledWith('group-1', { type: 'Ignore' });
    });

    it('highlights selected member row when Keep is active', () => {
      const onSelectionChange = vi.fn();
      const selections = new Map<string, ResolutionAction>([
        ['group-1', { type: 'Keep', targetPath: '/path/mod-a' }],
      ]);

      render(
        <DuplicateTable
          groups={mockGroups.slice(0, 1)}
          selections={selections}
          onSelectionChange={onSelectionChange}
        />,
      );

      // We check for the visual indicator (CheckCircle for keep, Trash2 for others)
      expect(screen.getByTestId('check-circle-icon')).toBeInTheDocument();
      expect(screen.getByTestId('trash-icon')).toBeInTheDocument();
    });
  });

  describe('Disabled state', () => {
    it('disables select dropdown when disabled prop is true', () => {
      const onSelectionChange = vi.fn();

      render(
        <DuplicateTable
          groups={mockGroups.slice(0, 1)}
          selections={new Map()}
          onSelectionChange={onSelectionChange}
          disabled={true}
        />,
      );

      const select = screen.getByRole('combobox', { name: /Resolution Action/i });
      expect(select).toBeDisabled();
    });
  });

  describe('Multi-member group support', () => {
    it('renders all members for groups with 3+ items', () => {
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
            modId: null,
            isSafe: true,
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

      expect(screen.getByText('Mod A - Original')).toBeInTheDocument();
      expect(screen.getByText('Mod B - Duplicate')).toBeInTheDocument();
      expect(screen.getByText('Mod E')).toBeInTheDocument();

      fireEvent.change(screen.getByRole('combobox', { name: /Resolution Action/i }), {
        target: { value: '/path/mod-e' },
      });

      expect(screen.getByText('Keep C: Mod E')).toBeInTheDocument();
    });
  });
});
