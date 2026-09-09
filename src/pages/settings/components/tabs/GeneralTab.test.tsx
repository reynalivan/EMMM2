import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import GeneralTab from './GeneralTab';
import type { CustomTheme, ThemeMetadata } from '../../../../shared/api/tauri/bindings';

let mockAutoClose = false;
let mockTheme = 'dark';
let mockCustomThemes: ThemeMetadata[] = [];
const mockSetAutoClose = vi.fn();
const mockUpdateThemeMutate = vi.fn();
const mockUpdateLanguageMutate = vi.fn();
const mockRefreshCustomThemes = vi.fn();
const mockAddToast = vi.fn();
const mockImportCustomTheme = vi.fn();
const mockExportCustomTheme = vi.fn();

vi.mock('../../../../shared/api/tauri/bindings', () => ({
  commands: {
    deleteCustomTheme: vi.fn(),
    importCustomTheme: (...args: unknown[]) => mockImportCustomTheme(...args),
    exportCustomTheme: (...args: unknown[]) => mockExportCustomTheme(...args),
  },
}));

vi.mock('../../hooks/useCustomThemes', () => ({
  useCustomThemes: () => ({
    customThemes: mockCustomThemes,
    refreshCustomThemes: mockRefreshCustomThemes,
  }),
}));

vi.mock('@/shared/ui/toast', () => ({
  useToastStore: () => ({ addToast: mockAddToast }),
}));

vi.mock('../../hooks/useSettings', () => ({
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
        game_focus_only: false,
        cooldown_ms: 150,
        next_preset: '',
        prev_preset: '',
        next_variant: '',
        prev_variant: '',
        toggle_overlay: '',
      },
      keyviewer: {
        enabled: false,
        status_ttl_seconds: 4,
        overlay_toggle_key: '',
        keybinds_dir: '',
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
    mockImportCustomTheme.mockReset();
    mockExportCustomTheme.mockReset();
  });

  it('renders Appearance and System sections', () => {
    render(<GeneralTab />);
    expect(screen.getByText('Appearance')).toBeInTheDocument();
    expect(screen.getByText('System Information')).toBeInTheDocument();
  });

  it('toggles Auto-Close launcher setting', () => {
    render(<GeneralTab />);

    // It starts with our mocked false value
    const toggle = screen.getByRole('checkbox', { name: /Auto-Close on Launch/i });
    expect(toggle).not.toBeChecked();

    fireEvent.click(toggle);

    // Should call store function with true
    expect(mockSetAutoClose).toHaveBeenCalledWith(true);
  });

  it('reflects initial store state on toggle', () => {
    mockAutoClose = true;
    render(<GeneralTab />);

    const toggle = screen.getByRole('checkbox', { name: /Auto-Close on Launch/i });
    expect(toggle).toBeChecked();
  });

  it('updates only theme when user selects a new option', () => {
    render(<GeneralTab />);

    const select = screen.getByRole('combobox', { name: 'Theme Selection' });
    fireEvent.change(select, { target: { value: 'light' } });

    expect(mockUpdateThemeMutate).toHaveBeenCalledWith('light');
  });

  it('imports themes through Rust and refreshes after success', async () => {
    const importedTheme: CustomTheme = {
      id: 'ocean-night',
      label: 'Ocean Night',
      config: { colors: {}, glass: {} },
    };
    mockImportCustomTheme.mockResolvedValueOnce(importedTheme);
    render(<GeneralTab />);

    fireEvent.click(screen.getByRole('button', { name: 'Import Theme' }));

    await waitFor(() => expect(mockImportCustomTheme).toHaveBeenCalledWith());
    expect(mockRefreshCustomThemes).toHaveBeenCalledTimes(1);
    expect(mockAddToast).toHaveBeenCalledWith('success', expect.stringContaining('Ocean Night'));
  });

  it('treats a cancelled Rust import as a no-op', async () => {
    mockImportCustomTheme.mockResolvedValueOnce(null);
    render(<GeneralTab />);

    fireEvent.click(screen.getByRole('button', { name: 'Import Theme' }));

    await waitFor(() => expect(mockImportCustomTheme).toHaveBeenCalledWith());
    expect(mockRefreshCustomThemes).not.toHaveBeenCalled();
    expect(mockAddToast).not.toHaveBeenCalled();
  });

  it('exports themes through Rust without reading or writing files in React', async () => {
    const exportedTheme: CustomTheme = {
      id: 'ocean-night',
      label: 'Ocean Night',
      config: { colors: {}, glass: {} },
    };
    mockTheme = exportedTheme.id;
    mockCustomThemes = [{ id: exportedTheme.id, label: exportedTheme.label }];
    mockExportCustomTheme.mockResolvedValueOnce('ocean-night.json');
    render(<GeneralTab />);

    fireEvent.click(screen.getByRole('button', { name: 'Export' }));

    await waitFor(() => expect(mockExportCustomTheme).toHaveBeenCalledWith('ocean-night'));
    expect(mockAddToast).toHaveBeenCalledWith(
      'success',
      expect.stringContaining('ocean-night.json'),
    );
  });
});
