import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import SettingsPage from './SettingsPage';

// Mock child components
vi.mock('./components/tabs/GamesTab', () => ({
  default: () => <div data-testid="games-tab">GamesTab</div>,
}));
vi.mock('./components/tabs/CatalogTab', () => ({
  default: () => <div data-testid="catalog-tab">CatalogTab</div>,
}));
vi.mock('./components/tabs/PrivacyTab', () => ({
  default: () => <div data-testid="privacy-tab">PrivacyTab</div>,
}));
vi.mock('./components/tabs/MaintenanceTab', () => ({
  default: () => <div data-testid="maintenance-tab">MaintenanceTab</div>,
}));
vi.mock('./components/tabs/GeneralTab', () => ({
  default: () => <div data-testid="general-tab">GeneralTab</div>,
}));
vi.mock('./components/tabs/LogsTab', () => ({
  default: () => <div data-testid="logs-tab">LogsTab</div>,
}));
vi.mock('./components/tabs/AITab', () => ({
  default: () => <div data-testid="ai-tab">AITab</div>,
}));
vi.mock('./components/tabs/IntegrationsTab', () => ({
  default: () => <div data-testid="integrations-tab">IntegrationsTab</div>,
}));

// Mock hooks
const mockSetSettingsTab = vi.fn();
let mockSettingsTab = 'general';
vi.mock('@/app/store', () => ({
  useAppStore: (selector: (state: Record<string, unknown>) => unknown) =>
    selector({
      settingsTab: mockSettingsTab,
      setSettingsTab: mockSetSettingsTab,
    }),
}));

let mockIsLoading = false;
let mockError: string | null = null;
vi.mock('@/entities/settings', () => ({
  useSettings: () => ({
    isLoading: mockIsLoading,
    error: mockError,
  }),
}));

describe('SettingsPage (TC-04)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockIsLoading = false;
    mockError = null;
    mockSettingsTab = 'general';
  });

  it('shows loading state initially', () => {
    mockIsLoading = true;
    render(<SettingsPage />);
    expect(screen.getByText('Loading...')).toBeInTheDocument();
  });

  it('shows error state if settings fail to load', () => {
    mockError = 'Failed to load DB';
    render(<SettingsPage />);
    expect(screen.getByText(/Failed to load DB/)).toBeInTheDocument();
  });

  it('renders default General tab and allows navigation', () => {
    render(<SettingsPage />);

    expect(screen.queryByRole('heading', { name: 'Settings' })).not.toBeInTheDocument();

    // Default tab
    expect(screen.getByTestId('general-tab')).toBeInTheDocument();

    // Click Games
    fireEvent.click(screen.getByRole('button', { name: 'Games' }));
    expect(screen.getByTestId('games-tab')).toBeInTheDocument();
    expect(screen.queryByTestId('general-tab')).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Catalog Assets' }));
    expect(screen.getByTestId('catalog-tab')).toBeInTheDocument();

    // Click Maintenance
    fireEvent.click(screen.getByRole('button', { name: 'Maintenance' }));
    expect(screen.getByTestId('maintenance-tab')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Integrations' }));
    expect(screen.getByTestId('integrations-tab')).toBeInTheDocument();
  });

  it('provides a compact section picker for mobile layouts', () => {
    render(<SettingsPage />);

    fireEvent.change(screen.getByRole('combobox', { name: 'Settings' }), {
      target: { value: 'games' },
    });

    expect(screen.getByTestId('games-tab')).toBeInTheDocument();
    expect(mockSetSettingsTab).toHaveBeenCalledWith('games');
  });

  it('does not expose the retired Updates section', () => {
    render(<SettingsPage />);

    expect(screen.queryByRole('button', { name: 'Updates' })).not.toBeInTheDocument();
    expect(screen.queryByRole('option', { name: 'Updates' })).not.toBeInTheDocument();
  });

  it('migrates a saved Updates tab selection to General', () => {
    mockSettingsTab = 'updates';
    render(<SettingsPage />);

    expect(screen.getByTestId('general-tab')).toBeInTheDocument();
    expect(mockSetSettingsTab).toHaveBeenCalledWith('general');
  });
});
