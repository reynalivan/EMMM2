import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import GeneralTab from './GeneralTab';
import type { ThemeMetadata } from '../../../../shared/api/tauri/bindings';

let mockAutoClose = false;
let mockTheme = 'dark';
let mockCustomThemes: ThemeMetadata[] = [];
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

vi.mock('../../hooks/useCustomThemes', () => ({
  useCustomThemes: () => ({
    customThemes: mockCustomThemes,
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

describe('GeneralTab (TC-04)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockAutoClose = false;
    mockTheme = 'dark';
    mockCustomThemes = [];
  });

  it('keeps General settings focused on user-facing preferences', () => {
    render(<GeneralTab />);
    expect(screen.getByText('Appearance')).toBeInTheDocument();
    expect(screen.getByText('System')).toBeInTheDocument();
    expect(screen.queryByText('Tauri Version')).not.toBeInTheDocument();
    expect(screen.queryByText('Database')).not.toBeInTheDocument();
    expect(screen.queryByText('Engine')).not.toBeInTheDocument();
    expect(screen.queryByText('Theme Information')).not.toBeInTheDocument();
  });

  it('toggles Auto-Close launcher setting', () => {
    render(<GeneralTab />);

    // It starts with our mocked false value
    const toggle = screen.getByRole('checkbox', { name: /Close after launch/i });
    expect(toggle).not.toBeChecked();

    fireEvent.click(toggle);

    // Should call store function with true
    expect(mockSetAutoClose).toHaveBeenCalledWith(true);
  });

  it('reflects initial store state on toggle', () => {
    mockAutoClose = true;
    render(<GeneralTab />);

    const toggle = screen.getByRole('checkbox', { name: /Close after launch/i });
    expect(toggle).toBeChecked();
  });

  it('updates only theme when user selects a new option', () => {
    render(<GeneralTab />);

    const select = screen.getByRole('combobox', { name: 'Theme' });
    fireEvent.change(select, { target: { value: 'light' } });

    expect(mockUpdateThemeMutate).toHaveBeenCalledWith('light');
  });

  it('lists custom themes in the same theme selector', () => {
    mockCustomThemes = [{ id: 'ocean-night', label: 'Ocean Night' }];
    render(<GeneralTab />);

    expect(screen.getByRole('option', { name: 'Ocean Night' })).toBeInTheDocument();
  });

  it('opens the privacy policy and terms of use from System', () => {
    render(<GeneralTab />);

    fireEvent.click(screen.getByRole('button', { name: /Privacy Policy/i }));
    expect(screen.getByRole('dialog')).toHaveTextContent(
      'EMMM does not read or transmit website passwords',
    );

    fireEvent.click(screen.getByRole('button', { name: 'Close dialog' }));
    fireEvent.click(screen.getByRole('button', { name: /Terms of Use/i }));
    expect(screen.getByRole('dialog')).toHaveTextContent('independent third-party utility');
  });

  it('opens the Ko-fi support page from General settings', async () => {
    render(<GeneralTab />);

    fireEvent.click(screen.getByRole('button', { name: 'Buy me a coffee' }));

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('browser_open_externally', {
        url: 'https://ko-fi.com/reynalivan',
      }),
    );
  });

  it('keeps application updates within System', () => {
    render(<GeneralTab />);

    fireEvent.click(screen.getByRole('button', { name: 'Check for Updates' }));

    expect(screen.getByText('Application Updates')).toBeInTheDocument();
    expect(mockCheckForUpdate).toHaveBeenCalledOnce();
  });
});
