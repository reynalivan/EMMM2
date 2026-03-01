import { describe, it, expect, vi, beforeEach, beforeAll } from 'vitest';
import { render, screen, act, fireEvent, waitFor } from '@testing-library/react';
import ConflictResolveDialog from './ConflictResolveDialog';
import { useAppStore } from '../../stores/useAppStore';
import { invoke } from '@tauri-apps/api/core';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  convertFileSrc: vi.fn((path) => path),
}));

vi.mock('../../../stores/useToastStore', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

describe('ConflictResolveDialog (TC-39)', () => {
  let queryClient: QueryClient;

  beforeAll(() => {
    // Mock HTMLDialogElement methods for jsdom
    HTMLDialogElement.prototype.showModal = vi.fn(function mock(this: HTMLDialogElement) {
      this.open = true;
    });
    HTMLDialogElement.prototype.close = vi.fn(function mock(this: HTMLDialogElement) {
      this.open = false;
    });
  });

  beforeEach(() => {
    vi.clearAllMocks();
    queryClient = new QueryClient();
    useAppStore.setState({
      conflictDialog: {
        open: false,
        conflict: null,
      },
      closeConflictDialog: () =>
        useAppStore.setState({ conflictDialog: { open: false, conflict: null } }),
    });
  });

  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );

  const openDialog = () => {
    useAppStore.setState({
      conflictDialog: {
        open: true,
        conflict: {
          type: 'RenameConflict',
          base_name: 'TestMod',
          existing_path: 'C:\\Mods\\DISABLED TestMod',
          attempted_target: 'C:\\Mods\\TestMod',
        },
      },
    });
  };

  it('TC-39-001: Mounts and fetches details on open', async () => {
    (invoke as ReturnType<typeof vi.fn>).mockImplementation((cmd) => {
      if (cmd === 'get_conflict_details') {
        return Promise.resolve({
          enabled: {
            path: 'C:\\Mods\\TestMod',
            folder_name: 'TestMod',
            is_enabled: true,
            total_size: 1024,
            file_count: 2,
            files: [{ name: 'main.ini', size: 500, is_ini: true }],
            thumbnail_path: null,
          },
          disabled: {
            path: 'C:\\Mods\\DISABLED TestMod',
            folder_name: 'DISABLED TestMod',
            is_enabled: false,
            total_size: 2048,
            file_count: 5,
            files: [{ name: 'old.ini', size: 500, is_ini: true }],
            thumbnail_path: null,
          },
        });
      }
      return Promise.resolve();
    });

    render(<ConflictResolveDialog />, { wrapper });

    act(() => {
      openDialog();
    });

    // Check header
    expect(screen.getByText('Name Conflict Detected')).toBeInTheDocument();

    // Check fetching logic
    await waitFor(() => {
      expect(screen.getByText('Enabled Version')).toBeInTheDocument();
      expect(screen.getByText('Disabled Version')).toBeInTheDocument();
    });

    expect(invoke).toHaveBeenCalledWith('get_conflict_details', {
      enabledPath: 'C:\\Mods\\TestMod',
      disabledPath: 'C:\\Mods\\DISABLED TestMod',
    });
  });

  it('TC-39-002: Resolve with Keep Enabled', async () => {
    (invoke as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
      enabled: { folder_name: 'E', files: [], total_size: 0, file_count: 0 },
      disabled: { folder_name: 'D', files: [], total_size: 0, file_count: 0 },
    });

    render(<ConflictResolveDialog />, { wrapper });

    act(() => openDialog());

    await waitFor(() => screen.getByText('Keep Enabled'));

    (invoke as ReturnType<typeof vi.fn>).mockResolvedValueOnce(undefined); // for resolve_conflict

    act(() => {
      fireEvent.click(screen.getByText(/Keep Enabled/));
    });

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('resolve_conflict', {
        keepPath: 'C:\\Mods\\TestMod',
        duplicatePath: 'C:\\Mods\\DISABLED TestMod',
        strategy: 'keep_enabled',
      });
      expect(useAppStore.getState().conflictDialog.open).toBe(false);
    });
  });

  it('TC-39-003: Resolve with Keep Disabled', async () => {
    (invoke as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
      enabled: { folder_name: 'E', files: [], total_size: 0, file_count: 0 },
      disabled: { folder_name: 'D', files: [], total_size: 0, file_count: 0 },
    });

    render(<ConflictResolveDialog />, { wrapper });

    act(() => openDialog());

    await waitFor(() => screen.getByText('Keep Disabled'));

    (invoke as ReturnType<typeof vi.fn>).mockResolvedValueOnce(undefined);

    act(() => {
      fireEvent.click(screen.getByText(/Keep Disabled/));
    });

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('resolve_conflict', {
        keepPath: 'C:\\Mods\\DISABLED TestMod',
        duplicatePath: 'C:\\Mods\\TestMod',
        strategy: 'keep_disabled',
      });
    });
  });

  it('TC-39-004: Resolve with Separate', async () => {
    (invoke as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
      enabled: { folder_name: 'E', files: [], total_size: 0, file_count: 0 },
      disabled: { folder_name: 'D', files: [], total_size: 0, file_count: 0 },
    });

    render(<ConflictResolveDialog />, { wrapper });

    act(() => openDialog());

    await waitFor(() => screen.getByText('Treat as Two Separate Mods'));

    (invoke as ReturnType<typeof vi.fn>).mockResolvedValueOnce(undefined);

    act(() => {
      fireEvent.click(screen.getByText('Treat as Two Separate Mods'));
    });

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('resolve_conflict', {
        keepPath: 'C:\\Mods\\TestMod', // Not strictly used for separate since both kept but passed by logic
        duplicatePath: 'C:\\Mods\\DISABLED TestMod',
        strategy: 'separate',
      });
    });
  });
});
