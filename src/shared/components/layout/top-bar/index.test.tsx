import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import TopBar from './index';

const mockSetWorkspaceView = vi.fn();

vi.mock('../../../../stores/useAppStore', () => ({
  useAppStore: (selector: (state: Record<string, unknown>) => unknown) =>
    selector({ workspaceView: 'dashboard', setWorkspaceView: mockSetWorkspaceView }),
}));

vi.mock('../../../../features/dashboard/hooks/useActiveGame', () => ({
  useActiveGame: () => ({ activeGame: null }),
}));

vi.mock('../../../../core/tauri/bindings', () => ({ commands: { launchGame: vi.fn() } }));
vi.mock('./GameSelector', () => ({ default: () => null }));
vi.mock('./ContextControls', () => ({ default: () => null }));
vi.mock('./GlobalActions', () => ({ default: () => null }));

describe('TopBar app menu', () => {
  beforeEach(() => vi.clearAllMocks());

  it('navigates to Mod Inbox from the app menu', () => {
    render(<TopBar />);

    fireEvent.click(screen.getByTitle('App Menu'));
    fireEvent.click(screen.getByTestId('nav-mod-inbox'));

    expect(mockSetWorkspaceView).toHaveBeenCalledWith('mod-inbox');
  });
});
