import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import SettingsPage from './SettingsPage';

// Mock child components
vi.mock('./tabs/GamesTab', () => ({ default: () => <div data-testid="games-tab">GamesTab</div> }));
vi.mock('./tabs/PrivacyTab', () => ({
  default: () => <div data-testid="privacy-tab">PrivacyTab</div>,
}));
vi.mock('./tabs/MaintenanceTab', () => ({
  default: () => <div data-testid="maintenance-tab">MaintenanceTab</div>,
}));
vi.mock('./tabs/GeneralTab', () => ({
  default: () => <div data-testid="general-tab">GeneralTab</div>,
}));
vi.mock('./tabs/LogsTab', () => ({ default: () => <div data-testid="logs-tab">LogsTab</div> }));
vi.mock('./tabs/AITab', () => ({ default: () => <div data-testid="ai-tab">AITab</div> }));
vi.mock('./tabs/UpdateTab', () => ({
  default: () => <div data-testid="update-tab">UpdateTab</div>,
}));

// Mock hooks
const mockSetWorkspaceView = vi.fn();
vi.mock('../../stores/useAppStore', () => ({
  useAppStore: () => ({
    setWorkspaceView: mockSetWorkspaceView,
  }),
}));

let mockIsLoading = false;
let mockError: string | null = null;
vi.mock('../../hooks/useSettings', () => ({
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
  });

  it('shows loading state initially', () => {
    mockIsLoading = true;
    render(<SettingsPage />);
    expect(screen.getByText('Loading settings...')).toBeInTheDocument();
  });

  it('shows error state if settings fail to load', () => {
    mockError = 'Failed to load DB';
    render(<SettingsPage />);
    expect(screen.getByText('Error loading settings: Failed to load DB')).toBeInTheDocument();
  });

  it('renders default General tab and allows navigation', () => {
    render(<SettingsPage />);

    // Header
    expect(screen.getByRole('heading', { name: 'Settings' })).toBeInTheDocument();

    // Default tab
    expect(screen.getByTestId('general-tab')).toBeInTheDocument();

    // Click Games
    fireEvent.click(screen.getByRole('button', { name: 'Games' }));
    expect(screen.getByTestId('games-tab')).toBeInTheDocument();
    expect(screen.queryByTestId('general-tab')).not.toBeInTheDocument();

    // Click Maintenance
    fireEvent.click(screen.getByRole('button', { name: 'Maintenance' }));
    expect(screen.getByTestId('maintenance-tab')).toBeInTheDocument();
  });

  it('handles back button correctly', () => {
    render(<SettingsPage />);
    // The back button is the first button before the header
    const backBtn = screen.getByRole('button', { name: '' });
    fireEvent.click(backBtn);
    expect(mockSetWorkspaceView).toHaveBeenCalledWith('dashboard');
  });
});
