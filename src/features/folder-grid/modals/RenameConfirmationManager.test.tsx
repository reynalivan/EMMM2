import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { useAppStore } from '../../../stores/useAppStore';
import RenameConfirmationManager from './RenameConfirmationManager';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('RenameConfirmationManager', () => {
  beforeAll(() => {
    HTMLDialogElement.prototype.showModal = vi.fn(function mock(this: HTMLDialogElement) {
      this.open = true;
    });
    HTMLDialogElement.prototype.close = vi.fn(function mock(this: HTMLDialogElement) {
      this.open = false;
    });
  });

  beforeEach(() => {
    vi.clearAllMocks();
    useAppStore.setState({
      activeGameId: 'game-1',
      workspaceDialogState: { kind: 'renameConfirmations' },
      renameConfirmationsByGame: {
        'game-1': [
          {
            group_id: 'rename-group-1',
            kind: 'Mod',
            reason: 'MissingIdentity',
            scope_key: 'alice',
            previous_paths: ['Alice/Old'],
            current_paths: ['Alice/New'],
            previous_path_count: 1,
            current_path_count: 1,
            candidates_truncated: false,
          },
        ],
      },
    });
  });

  it('does not mount a blocking modal until rename review is requested', () => {
    useAppStore.setState({ workspaceDialogState: { kind: 'none' } });

    render(<RenameConfirmationManager />, {
      wrapper: ({ children }) => (
        <QueryClientProvider client={new QueryClient()}>{children}</QueryClientProvider>
      ),
    });

    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('requires an explicit decision and submits the reviewed rename mapping', async () => {
    (invoke as ReturnType<typeof vi.fn>).mockImplementation((command: string) => {
      if (command === 'resolve_rename_confirmations') {
        return Promise.resolve({
          game_id: 'game-1',
          reason: 'ManualRepair',
          status: 'Applied',
          folder_conflicts: [],
          rename_confirmations: [],
          error_message: null,
          changed_roots: [],
          objects_changed: false,
          folders_changed: true,
          collections_changed: true,
          runtime_file_changed: false,
          overlay_refresh_triggered: false,
          thumbnail_roots: [],
          cleared_selection_paths: [],
          path_updates: [{ from: 'Alice/Old', to: 'Alice/New', kind: 'Mod' }],
          collection_reference_impact: {
            affected_collection_count: 1,
            affected_collection_names: ['Preset'],
            rewritten_paths: [{ from: 'Alice/Old', to: 'Alice/New' }],
            missing_paths: [],
          },
          change_summary: {
            object_changes: { added: 0, removed: 0, renamed: 0, modified: 0 },
            mod_changes: { added: 0, removed: 0, renamed: 1, modified: 0 },
            object_sample_names: [],
            mod_sample_names: ['New'],
            has_user_visible_changes: true,
          },
        });
      }
      return Promise.resolve();
    });

    render(<RenameConfirmationManager />, {
      wrapper: ({ children }) => (
        <QueryClientProvider client={new QueryClient()}>{children}</QueryClientProvider>
      ),
    });

    const applyButton = screen.getByRole('button', { name: 'Apply all decisions' });
    expect(applyButton).toBeDisabled();
    fireEvent.click(screen.getByRole('radio', { name: /This folder was renamed or moved/i }));
    expect(applyButton).toBeEnabled();
    fireEvent.click(applyButton);

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('resolve_rename_confirmations', {
        gameId: 'game-1',
        resolutions: [
          {
            group_id: 'rename-group-1',
            action: 'Rename',
            previous_path: 'Alice/Old',
            current_path: 'Alice/New',
          },
        ],
      });
    });
  });
});
