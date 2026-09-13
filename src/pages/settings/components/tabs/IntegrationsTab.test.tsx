import { fireEvent, render, screen, waitFor } from '@/tests/testing/test-utils';
import { useQuery } from '@tanstack/react-query';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { describe, beforeEach, expect, it, vi } from 'vitest';
import IntegrationsTab from './IntegrationsTab';

const mockMutate = vi.fn();
const mockMutateAsync = vi.fn();
const mockAddToast = vi.fn();
let configuredExecutable: string | null = null;

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
}));

vi.mock('@/entities/settings', () => ({
  useSettings: () => ({
    settings: {
      external_tools: {
        mod_viewer_executable: configuredExecutable,
      },
    },
    setModViewerExecutable: {
      mutate: mockMutate,
      mutateAsync: mockMutateAsync,
      isPending: false,
    },
  }),
}));

vi.mock('@/shared/api/tauri/bindings', () => ({
  commands: {
    browserOpenExternally: vi.fn(),
    checkPathExistsCmd: vi.fn(),
  },
}));

vi.mock('@/shared/ui/toast', () => ({
  useToastStore: () => ({ addToast: mockAddToast }),
}));

describe('IntegrationsTab', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    configuredExecutable = null;
    mockMutateAsync.mockResolvedValue(undefined);
    vi.mocked(useQuery).mockReturnValue({
      data: null,
      isLoading: false,
      error: null,
    } as unknown as ReturnType<typeof useQuery>);
  });

  it('starts unconfigured and only saves a selected executable after disclosure acceptance', async () => {
    vi.mocked(openDialog).mockResolvedValue('C:/Tools/Mod Viewer.exe');
    render(<IntegrationsTab />);

    expect(screen.getByTestId('mod-viewer-status')).toHaveTextContent('Not configured');

    fireEvent.click(screen.getByRole('button', { name: 'Select executable' }));
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(mockMutateAsync).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole('button', { name: 'Select executable' }));
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());
    fireEvent.click(screen.getByRole('button', { name: 'Continue' }));

    await waitFor(() => expect(mockMutateAsync).toHaveBeenCalledWith('C:/Tools/Mod Viewer.exe'));
  });

  it('shows a missing configured executable without clearing it', () => {
    configuredExecutable = 'C:/Missing/Mod Viewer.exe';
    vi.mocked(useQuery).mockReturnValue({
      data: false,
      isLoading: false,
      error: null,
    } as unknown as ReturnType<typeof useQuery>);

    render(<IntegrationsTab />);

    expect(screen.getByTestId('mod-viewer-status')).toHaveTextContent(
      'Configured executable was not found',
    );
    expect(screen.getByTestId('mod-viewer-executable-path')).toHaveTextContent(
      'C:/Missing/Mod Viewer.exe',
    );
    expect(mockMutate).not.toHaveBeenCalled();
  });

  it('opens the GitHub latest-release page instead of downloading a binary', async () => {
    const { commands } = await import('@/shared/api/tauri/bindings');
    render(<IntegrationsTab />);

    fireEvent.click(screen.getByRole('button', { name: 'View latest release' }));

    await waitFor(() =>
      expect(commands.browserOpenExternally).toHaveBeenCalledWith(
        'https://github.com/drelymk/mod_viewer/releases/latest',
      ),
    );
  });

  it('clears only the configuration when Remove is selected', () => {
    configuredExecutable = 'C:/Tools/Mod Viewer.exe';
    render(<IntegrationsTab />);

    fireEvent.click(screen.getByRole('button', { name: 'Remove' }));

    expect(mockMutate).toHaveBeenCalledWith(null);
  });
});
