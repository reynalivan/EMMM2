import { fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
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
});
