import { act, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { commands } from '@/shared/api/tauri/bindings';
import { openObjectClassificationWizard } from '@/features/import-batches';
import { DashboardIdentitySuggestions } from './DashboardIdentitySuggestions';

const status = {
  state: 'catalog_missing',
  catalogId: null,
  catalogVersion: null,
  suggestedCount: 0,
  checkedCount: 0,
  totalCount: 0,
  failedCount: 0,
  hasKeyviewerTargets: false,
  message: null,
};

vi.mock('@tanstack/react-query', () => ({
  useQuery: () => ({ data: status }),
  useQueryClient: () => ({}),
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

vi.mock('@/shared/api/tauri/bindings', () => ({
  commands: {
    getObjectIdentitySuggestionStatus: vi.fn(),
    retryObjectIdentitySuggestions: vi.fn(),
    listObjectIdentitySuggestions: vi.fn(),
    dismissObjectIdentitySuggestion: vi.fn(),
  },
}));

vi.mock('@/shared/lib/queryRefresh', () => ({
  publishQueryInvalidations: vi.fn(),
}));

vi.mock('@/features/import-batches', () => ({
  openObjectClassificationWizard: vi.fn(),
}));

describe('DashboardIdentitySuggestions', () => {
  beforeEach(() => {
    status.state = 'catalog_missing';
    status.suggestedCount = 0;
    vi.mocked(commands.listObjectIdentitySuggestions).mockReset();
    vi.mocked(openObjectClassificationWizard).mockReset();
  });

  afterEach(() => {
    document.getElementById('workspace-main')?.remove();
  });

  it('portals the help overlay into the workspace main instead of the dashboard scroll owner', () => {
    const workspaceMain = document.createElement('main');
    workspaceMain.id = 'workspace-main';
    document.body.appendChild(workspaceMain);

    render(<DashboardIdentitySuggestions gameId="game-1" onOpenSettings={vi.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: 'identity_suggestions.how_to_action' }));

    const dialog = screen.getByRole('dialog');
    expect(dialog.parentElement).toBe(workspaceMain);
    expect(dialog).toHaveClass('modal-middle');
    expect(dialog.querySelector('.modal-box')).toHaveClass('overflow-hidden');
  });

  it('does not launch a review dismissed while suggestions are loading', async () => {
    status.state = 'ready';
    status.suggestedCount = 1;
    let resolvePage!: (page: { items: { objectId: string }[]; nextOffset: null }) => void;
    vi.mocked(commands.listObjectIdentitySuggestions).mockReturnValue(
      new Promise((resolve) => {
        resolvePage = resolve;
      }) as never,
    );
    render(<DashboardIdentitySuggestions gameId="game-1" onOpenSettings={vi.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: 'identity_suggestions.review_action' }));
    fireEvent.click(screen.getByRole('button', { name: 'identity_suggestions.not_now' }));
    await act(async () => {
      resolvePage({ items: [{ objectId: 'object-1' }], nextOffset: null });
    });

    expect(openObjectClassificationWizard).not.toHaveBeenCalled();
  });

  it('suppresses review failures after dismissal', async () => {
    status.state = 'ready';
    status.suggestedCount = 1;
    let rejectPage!: (error: Error) => void;
    vi.mocked(commands.listObjectIdentitySuggestions).mockReturnValue(
      new Promise((_, reject) => {
        rejectPage = reject;
      }) as never,
    );
    render(<DashboardIdentitySuggestions gameId="game-1" onOpenSettings={vi.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: 'identity_suggestions.review_action' }));
    fireEvent.click(screen.getByRole('button', { name: 'identity_suggestions.not_now' }));
    await act(async () => {
      rejectPage(new Error('obsolete review failure'));
      await Promise.resolve();
    });

    expect(openObjectClassificationWizard).not.toHaveBeenCalled();
  });

  it('does not launch suggestions after the dashboard game changes', async () => {
    status.state = 'ready';
    status.suggestedCount = 1;
    let resolvePage!: (page: { items: { objectId: string }[]; nextOffset: null }) => void;
    vi.mocked(commands.listObjectIdentitySuggestions).mockReturnValue(
      new Promise((resolve) => {
        resolvePage = resolve;
      }) as never,
    );
    const view = render(<DashboardIdentitySuggestions gameId="game-1" onOpenSettings={vi.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: 'identity_suggestions.review_action' }));
    view.rerender(<DashboardIdentitySuggestions gameId="game-2" onOpenSettings={vi.fn()} />);
    expect(
      screen.getByRole('button', { name: 'identity_suggestions.review_action' }),
    ).toBeEnabled();
    await act(async () => {
      resolvePage({ items: [{ objectId: 'object-1' }], nextOffset: null });
    });

    expect(openObjectClassificationWizard).not.toHaveBeenCalled();
  });
});
