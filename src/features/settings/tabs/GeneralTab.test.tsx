import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import GeneralTab from './GeneralTab';

let mockAutoClose = false;
const mockSetAutoClose = vi.fn();
const mockUpdateThemeMutate = vi.fn();
const mockUpdateLanguageMutate = vi.fn();

vi.mock('../../../hooks/useSettings', () => ({
  useSettings: () => ({
    settings: {
      theme: 'dark',
      language: 'en',
      games: [],
      active_game_id: null,
      safe_mode: {
        enabled: true,
        pin_hash: null,
        recovery_code_hash: null,
        keywords: [],
        force_exclusive_mode: false,
      },
      ai: {
        enabled: false,
        api_key: null,
        base_url: null,
      },
      hotkeys: {
        enabled: false,
        game_focus_only: false,
        cooldown_ms: 150,
        toggle_safe_mode: '',
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

vi.mock('../../../stores/useAppStore', () => ({
  useAppStore: () => ({
    autoCloseLauncher: mockAutoClose,
    setAutoCloseLauncher: mockSetAutoClose,
  }),
}));

describe('GeneralTab (TC-04)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockAutoClose = false;
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
});
