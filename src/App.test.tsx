import { render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { vi, describe, it, expect, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import App from './App';
import * as appStore from './stores/useAppStore';

// Mock the components so we don't render the whole app
vi.mock('./components/layout/MainLayout', () => ({
  default: () => <div data-testid="dashboard">Dashboard</div>,
}));
vi.mock('./features/onboarding/WelcomeScreen', () => ({
  default: () => <div data-testid="welcome">Welcome</div>,
}));
vi.mock('./features/folder-grid/ConflictResolveDialog', () => ({
  default: () => null,
}));
vi.mock('./components/ui/Toast', () => ({
  ToastContainer: () => null,
}));

// Mock logger
vi.mock('@tauri-apps/plugin-log', () => ({
  trace: vi.fn(),
  info: vi.fn(),
  error: vi.fn(),
  warn: vi.fn(),
  debug: vi.fn(),
  attachConsole: vi.fn(),
}));

describe('App Bootstrap Routing & Initialization (TC-01)', () => {
  beforeEach(() => {
    vi.clearAllMocks();

    // Mock the store to prevent initialization errors
    vi.spyOn(appStore.useAppStore.getState(), 'initStore').mockResolvedValue(undefined);
  });

  it('TC-01-08: Routes to /welcome on FreshInstall status', async () => {
    vi.mocked(invoke).mockImplementation((cmd) => {
      if (cmd === 'check_config_status') return Promise.resolve('FreshInstall');
      if (cmd === 'check_metadata_update') return Promise.resolve();
      return Promise.reject(new Error(`Unhandled mock command: ${cmd}`));
    });

    render(
      <MemoryRouter initialEntries={['/']}>
        <App />
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(screen.getByTestId('welcome')).toBeInTheDocument();
    });
    expect(invoke).toHaveBeenCalledWith('check_config_status');
  });

  it('TC-01-09: Routes to /dashboard on HasConfig status', async () => {
    vi.mocked(invoke).mockImplementation((cmd) => {
      if (cmd === 'check_config_status') return Promise.resolve('HasConfig');
      if (cmd === 'check_metadata_update') return Promise.resolve();
      return Promise.reject(new Error(`Unhandled mock command: ${cmd}`));
    });

    render(
      <MemoryRouter initialEntries={['/']}>
        <App />
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(screen.getByTestId('dashboard')).toBeInTheDocument();
    });
    expect(invoke).toHaveBeenCalledWith('check_config_status');
  });

  it('TC-01-10: Falls back to Dashboard on IPC timeout or error', async () => {
    vi.mocked(invoke).mockImplementation((cmd) => {
      if (cmd === 'check_config_status') return Promise.reject(new Error('Backend missing'));
      if (cmd === 'check_metadata_update') return Promise.resolve();
      return Promise.reject(new Error(`Unhandled mock command: ${cmd}`));
    });

    render(
      <MemoryRouter initialEntries={['/']}>
        <App />
      </MemoryRouter>,
    );

    await waitFor(() => {
      // Per implementation in App.tsx (Fallback for frontend-only dev mode)
      expect(screen.getByTestId('dashboard')).toBeInTheDocument();
    });
  });
});
