import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { DashboardQuickActions } from './DashboardQuickActions';

vi.mock('../../../core/tauri/bindings', () => ({ commands: { launchGame: vi.fn() } }));

describe('DashboardQuickActions', () => {
  it('opens Mod Inbox from its dashboard tile', () => {
    const setWorkspaceView = vi.fn();
    render(<DashboardQuickActions activeGameId="game-1" setWorkspaceView={setWorkspaceView} />);

    fireEvent.click(screen.getByRole('button', { name: 'Mod Inbox' }));

    expect(setWorkspaceView).toHaveBeenCalledWith('mod-inbox');
  });
});
