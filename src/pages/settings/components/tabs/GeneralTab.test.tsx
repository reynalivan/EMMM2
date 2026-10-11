import { act, render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import GeneralTab from './GeneralTab';
import type { ThemeMetadata } from '../../../../shared/api/tauri/bindings';

let mockAutoClose = false;
let mockTheme = 'dark';
let mockCustomThemes: ThemeMetadata[] = [];
let mockCustomThemesError: Error | null = null;
let mockCustomThemesRefreshing = false;
const mockRefreshCustomThemes = vi.fn();
const mockSetAutoClose = vi.fn();
const mockUpdateThemeMutate = vi.fn();
const mockUpdateLanguageMutate = vi.fn();
const mockSetTelemetryEnabledMutate = vi.fn();
const mockCheckForUpdate = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue({
    state: 'not_installed',
    pack_id: null,
    version: null,
    message: null,
    entries: 0,
  }),
}));

vi.mock('../../hooks/useAppUpdater', () => ({
  useAppUpdater: () => ({
    update: null,
    isChecking: false,
    isInstalling: false,
    progress: null,
    error: null,
    hasChecked: false,
    checkForUpdate: mockCheckForUpdate,
    downloadAndInstall: vi.fn(),
    dismiss: vi.fn(),
  }),
}));

vi.mock('@/entities/settings', () => ({
  useCustomThemes: () => ({
    customThemes: mockCustomThemes,
    customThemeIds:
      mockCustomThemesError || mockCustomThemesRefreshing
        ? undefined
        : mockCustomThemes.map((theme) => theme.id),
    error: mockCustomThemesError,
    refreshing: mockCustomThemesRefreshing,
    refreshCustomThemes: mockRefreshCustomThemes,
  }),
  useSettings: () => ({
    settings: {
      theme: mockTheme,
      language: 'en',
      games: [],
      active_game_id: null,
      safety: {
        keywords: [],
      },
      ai: {
        enabled: false,
        has_api_key: false,
        base_url: null,
      },
      hotkeys: {
        enabled: false,
        safe_mode: 'F5',
        next_preset: '',
        prev_preset: '',
        toggle_overlay: '',
      },
      keyviewer: {
        enabled: false,
      },
    },
    updateTheme: {
      mutate: mockUpdateThemeMutate,
      isPending: false,
    },
    updateLanguage: {
      mutate: mockUpdateLanguageMutate,
      isPending: false,
    },
    setTelemetryEnabled: {
      mutate: mockSetTelemetryEnabledMutate,
      isPending: false,
    },
  }),
}));

vi.mock('@/app/store', () => ({
  useAppStore: (selector: (state: Record<string, unknown>) => unknown) =>
    selector({ autoCloseLauncher: mockAutoClose, setAutoCloseLauncher: mockSetAutoClose }),
}));

async function renderGeneralTab() {
  let view!: ReturnType<typeof render>;
  await act(async () => {
    view = render(<GeneralTab />);
  });
  return view;
}

describe('GeneralTab (TC-04)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockAutoClose = false;
    mockTheme = 'dark';
    mockCustomThemes = [];
    mockCustomThemesError = null;
    mockCustomThemesRefreshing = false;
  });

  it('keeps General settings focused on user-facing preferences', async () => {
    await renderGeneralTab();
    expect(screen.getByText('Appearance')).toBeInTheDocument();
    expect(screen.getByText('System')).toBeInTheDocument();
    expect(screen.queryByText('Tauri Version')).not.toBeInTheDocument();
    expect(screen.queryByText('Database')).not.toBeInTheDocument();
    expect(screen.queryByText('Engine')).not.toBeInTheDocument();
    expect(screen.queryByText('Theme Information')).not.toBeInTheDocument();
  });

  it('toggles Auto-Close launcher setting', async () => {
    await renderGeneralTab();

    // It starts with our mocked false value
    const toggle = screen.getByRole('checkbox', { name: /Close after launch/i });
    expect(toggle).not.toBeChecked();

    fireEvent.click(toggle);

    // Should call store function with true
    expect(mockSetAutoClose).toHaveBeenCalledWith(true);
  });

  it('reflects initial store state on toggle', async () => {
    mockAutoClose = true;
    await renderGeneralTab();

    const toggle = screen.getByRole('checkbox', { name: /Close after launch/i });
    expect(toggle).toBeChecked();
  });

  it('updates only theme when user selects a new option', async () => {
    await renderGeneralTab();

    const select = screen.getByRole('combobox', { name: 'Theme' });
    fireEvent.change(select, { target: { value: 'light' } });

    expect(mockUpdateThemeMutate).toHaveBeenCalledWith('light');
  });

  it('lists custom themes in the same theme selector', async () => {
    mockCustomThemes = [{ id: 'ocean-night', label: 'Ocean Night' }];
    await renderGeneralTab();

    expect(screen.getByRole('option', { name: 'Ocean Night' })).toBeInTheDocument();
  });

  it.each(['pending', 'failed'] as const)(
    'shows the actual stored selection while custom theme discovery is %s',
    async (state) => {
      mockTheme = 'dark';
      mockCustomThemesRefreshing = state === 'pending';
      mockCustomThemesError =
        state === 'failed' ? new Error('Theme directory is unavailable') : null;
      await renderGeneralTab();

      await waitFor(() =>
        expect(screen.getByRole('combobox', { name: 'Theme' })).toHaveValue('dark'),
      );
      expect(mockUpdateThemeMutate).not.toHaveBeenCalled();
    },
  );

  it('selects onyx for legacy dark when no custom dark theme exists', async () => {
    await renderGeneralTab();

    expect(screen.getByRole('combobox', { name: 'Theme' })).toHaveValue('onyx');
  });

  it('preserves the selection of a custom theme named dark', async () => {
    mockCustomThemes = [{ id: 'dark', label: 'Custom Dark' }];
    await renderGeneralTab();

    expect(screen.getByRole('combobox', { name: 'Theme' })).toHaveValue('dark');
  });

  it('shows theme discovery failure and lets the user retry', async () => {
    mockCustomThemesError = new Error('Could not read custom theme dark: file is locked');
    const { rerender } = await renderGeneralTab();

    expect(screen.getByRole('alert')).toHaveTextContent(mockCustomThemesError.message);
    fireEvent.click(screen.getByRole('button', { name: 'Retry' }));
    expect(mockRefreshCustomThemes).toHaveBeenCalledTimes(1);

    mockCustomThemesRefreshing = true;
    await act(async () => {
      rerender(<GeneralTab />);
    });
    expect(screen.getByRole('button', { name: 'Loading...' })).toBeDisabled();
    mockCustomThemesError = null;
    mockCustomThemesRefreshing = false;
    await act(async () => {
      rerender(<GeneralTab />);
    });
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('opens the privacy policy and terms of use from System', async () => {
    await renderGeneralTab();

    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: /Privacy Policy/i }));
    });
    expect(screen.getByRole('dialog')).toHaveTextContent(
      'EMMM does not read or transmit website passwords',
    );

    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Close dialog' }));
    });
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: /Terms of Use/i }));
    });
    expect(screen.getByRole('dialog')).toHaveTextContent('independent third-party utility');
  });

  it('opens the Ko-fi support page from General settings', async () => {
    await renderGeneralTab();

    fireEvent.click(screen.getByRole('button', { name: 'Buy me a coffee' }));

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('browser_open_externally', {
        url: 'https://ko-fi.com/reynalivan',
      }),
    );
  });

  it('keeps application updates within System', async () => {
    await renderGeneralTab();

    fireEvent.click(screen.getByRole('button', { name: 'Check for Updates' }));

    expect(screen.getByText('Application Updates')).toBeInTheDocument();
    expect(mockCheckForUpdate).toHaveBeenCalledOnce();
  });
});
