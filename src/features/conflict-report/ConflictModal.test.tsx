import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { ConflictInfo } from '@/entities/workspace';
import ConflictModal from './ConflictModal';

const mocks = vi.hoisted(() => ({
  bulkToggle: vi.fn(),
  openInExplorer: vi.fn(),
  isPending: false,
}));

vi.mock('../../shared/lib/hooks/useDialogSync', () => ({
  useDialogSync: vi.fn(),
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, values?: Record<string, unknown>) => {
      const value = values?.name ?? values?.count ?? values?.error;
      return value === undefined ? key : `${key}:${String(value)}`;
    },
  }),
}));

vi.mock('../mod-runtime/hooks/useBulkModMutations', () => ({
  useBulkToggle: () => ({ mutateAsync: mocks.bulkToggle, isPending: mocks.isPending }),
}));

vi.mock('../../shared/api/tauri/bindings', () => ({
  commands: {
    openInExplorer: (...args: unknown[]) => mocks.openInExplorer(...args),
  },
}));

vi.mock('../../shared/lib/appError', () => ({
  formatAppError: (error: unknown) => String(error),
}));

const conflict: ConflictInfo = {
  hash: 'abcdef12',
  section_name: 'TextureOverrideBody',
  mod_paths: ['E:/Mods/ModA', 'E:/Mods/ModB'],
  is_active: true,
  kind: 'resource_hash',
  certainty: 'potential',
  has_conditional_evidence: true,
  evidence: [
    {
      mod_path: 'E:/Mods/ModA',
      source_path: 'E:/Mods/ModA/config.ini',
      section_name: 'TextureOverrideBody',
      namespace: 'Alice',
      condition: '$active',
      priority: 7,
      match_first_index: 0,
      shader_stage: null,
    },
    {
      mod_path: 'E:/Mods/ModB',
      source_path: 'E:/Mods/ModB/config.ini',
      section_name: 'TextureOverrideBody',
      namespace: null,
      condition: null,
      priority: null,
      match_first_index: 0,
      shader_stage: null,
    },
  ],
};

describe('ConflictModal', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.isPending = false;
    mocks.openInExplorer.mockResolvedValue(undefined);
    mocks.bulkToggle.mockResolvedValue({
      success: ['E:/Mods/DISABLED ModB'],
      failures: [],
      collection_impact: {
        affected_collection_count: 0,
        affected_collection_names: [],
        rewritten_paths: [],
        missing_paths: [],
      },
      path_rewrites: [{ old_path: 'E:/Mods/ModB', new_path: 'E:/Mods/DISABLED ModB' }],
    });
  });

  it('shows conflict kind and actionable evidence for enabled mods', () => {
    render(<ConflictModal open onClose={vi.fn()} conflicts={[conflict]} gameId="game-1" />);

    expect(screen.getByText('scanner:conflict_modal.kind.resource_hash')).toBeInTheDocument();
    expect(screen.getByText('E:/Mods/ModA')).toBeInTheDocument();
    expect(screen.getByText('E:/Mods/ModB')).toBeInTheDocument();
    expect(screen.getByText('E:/Mods/ModA/config.ini')).toBeInTheDocument();
    expect(screen.getByText('E:/Mods/ModB/config.ini')).toBeInTheDocument();
    expect(screen.getByText('scanner:conflict_modal.match_priority: 7')).toBeInTheDocument();
    expect(screen.getAllByText('scanner:conflict_modal.first_index: 0')).toHaveLength(2);
    expect(screen.getByText('$active')).toBeInTheDocument();
  });

  it('groups runtime hashes that involve the same mod locations', () => {
    const secondHash: ConflictInfo = {
      ...conflict,
      hash: '12345678',
      section_name: 'TextureOverrideHair',
      evidence: conflict.evidence.map((item) => ({
        ...item,
        section_name: 'TextureOverrideHair',
      })),
    };

    render(
      <ConflictModal open onClose={vi.fn()} conflicts={[conflict, secondHash]} gameId="game-1" />,
    );

    expect(screen.getByText('scanner:conflict_modal.runtime_keys:2')).toBeInTheDocument();
    expect(screen.getByText('scanner:conflict_modal.mod_locations:2')).toBeInTheDocument();
    expect(
      screen.getAllByRole('button', {
        name: 'scanner:conflict_modal.keep_enabled:ModA',
        hidden: true,
      }),
    ).toHaveLength(1);
  });

  it('does not preselect or mutate a potential conflict when opened', () => {
    render(<ConflictModal open onClose={vi.fn()} conflicts={[conflict]} gameId="game-1" />);

    expect(
      screen.getByRole('button', {
        name: 'scanner:conflict_modal.review_changes',
        hidden: true,
      }),
    ).toBeDisabled();
    expect(
      screen.getByRole('button', {
        name: 'scanner:conflict_modal.keep_enabled:ModA',
        hidden: true,
      }),
    ).toHaveAttribute('aria-pressed', 'false');
    expect(mocks.bulkToggle).not.toHaveBeenCalled();
  });

  it('reviews the impact before disabling non-winning mods', async () => {
    render(<ConflictModal open onClose={vi.fn()} conflicts={[conflict]} gameId="game-1" />);

    fireEvent.click(
      screen.getByRole('button', {
        name: 'scanner:conflict_modal.keep_enabled:ModA',
        hidden: true,
      }),
    );
    expect(
      screen.getByRole('button', {
        name: 'scanner:conflict_modal.disable_mod:ModB',
        hidden: true,
      }),
    ).toHaveAttribute('aria-pressed', 'true');

    fireEvent.click(
      screen.getByRole('button', {
        name: 'scanner:conflict_modal.review_changes',
        hidden: true,
      }),
    );
    expect(screen.getByText('scanner:conflict_modal.review_title')).toBeInTheDocument();
    expect(screen.getByText('ModB')).toBeInTheDocument();
    expect(screen.getByText('E:/Mods/ModB')).toBeInTheDocument();
    expect(mocks.bulkToggle).not.toHaveBeenCalled();

    fireEvent.click(
      screen.getByRole('button', {
        name: 'scanner:conflict_modal.disable_confirm:1',
        hidden: true,
      }),
    );

    await waitFor(() => {
      expect(mocks.bulkToggle).toHaveBeenCalledWith({
        gameId: 'game-1',
        paths: ['E:/Mods/ModB'],
        enable: false,
      });
    });
  });

  it('keeps failed paths selected and visible for retry after partial failure', async () => {
    mocks.bulkToggle.mockResolvedValue({
      success: [],
      failures: [{ path: 'E:/Mods/ModB', error: 'File is locked' }],
      collection_impact: {
        affected_collection_count: 0,
        affected_collection_names: [],
        rewritten_paths: [],
        missing_paths: [],
      },
      path_rewrites: [],
    });
    render(<ConflictModal open onClose={vi.fn()} conflicts={[conflict]} gameId="game-1" />);

    fireEvent.click(
      screen.getByRole('button', {
        name: 'scanner:conflict_modal.keep_enabled:ModA',
        hidden: true,
      }),
    );
    fireEvent.click(
      screen.getByRole('button', {
        name: 'scanner:conflict_modal.review_changes',
        hidden: true,
      }),
    );
    fireEvent.click(
      screen.getByRole('button', {
        name: 'scanner:conflict_modal.disable_confirm:1',
        hidden: true,
      }),
    );

    expect(await screen.findByText('File is locked')).toBeInTheDocument();
    expect(
      screen.getByRole('button', {
        name: 'scanner:conflict_modal.disable_mod:ModB',
        hidden: true,
      }),
    ).toHaveAttribute('aria-pressed', 'true');
  });

  it('opens a participating mod folder without changing decisions', () => {
    render(<ConflictModal open onClose={vi.fn()} conflicts={[conflict]} gameId="game-1" />);

    fireEvent.click(
      screen.getByRole('button', {
        name: 'scanner:conflict_modal.open_folder:ModA',
        hidden: true,
      }),
    );

    expect(mocks.openInExplorer).toHaveBeenCalledWith('game-1', 'E:/Mods/ModA');
    expect(mocks.bulkToggle).not.toHaveBeenCalled();
  });

  it('clears stale decisions when the dialog is closed externally and reopened', () => {
    const { rerender } = render(
      <ConflictModal open onClose={vi.fn()} conflicts={[conflict]} gameId="game-1" />,
    );
    fireEvent.click(
      screen.getByRole('button', {
        name: 'scanner:conflict_modal.keep_enabled:ModA',
        hidden: true,
      }),
    );
    expect(
      screen.getByRole('button', {
        name: 'scanner:conflict_modal.review_changes',
        hidden: true,
      }),
    ).toBeEnabled();

    rerender(
      <ConflictModal open={false} onClose={vi.fn()} conflicts={[conflict]} gameId="game-1" />,
    );
    rerender(<ConflictModal open onClose={vi.fn()} conflicts={[conflict]} gameId="game-1" />);

    expect(
      screen.getByRole('button', {
        name: 'scanner:conflict_modal.review_changes',
        hidden: true,
      }),
    ).toBeDisabled();
  });

  it('keeps the review available and shows an actionable command error', async () => {
    mocks.bulkToggle.mockRejectedValue(new Error('Command failed'));
    render(<ConflictModal open onClose={vi.fn()} conflicts={[conflict]} gameId="game-1" />);
    fireEvent.click(
      screen.getByRole('button', {
        name: 'scanner:conflict_modal.keep_enabled:ModA',
        hidden: true,
      }),
    );
    fireEvent.click(
      screen.getByRole('button', {
        name: 'scanner:conflict_modal.review_changes',
        hidden: true,
      }),
    );
    fireEvent.click(
      screen.getByRole('button', {
        name: 'scanner:conflict_modal.disable_confirm:1',
        hidden: true,
      }),
    );

    expect(
      await screen.findByText('scanner:conflict_modal.submit_failed:Error: Command failed'),
    ).toBeInTheDocument();
    expect(
      screen.getByRole('button', {
        name: 'scanner:conflict_modal.disable_confirm:1',
        hidden: true,
      }),
    ).toBeEnabled();
  });
});
