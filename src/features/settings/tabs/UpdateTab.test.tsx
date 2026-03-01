/**
 * Tests for UpdateTab component.
 * Covers: TC-34-001 (No Update Available), TC-34-002 (Update Available UI),
 *         TC-34-003 (Check Button Interaction), TC-34-004 (Download Progress),
 *         TC-34-005 (Update Error), TC-34-006 (Metadata Sync)
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '../../../testing/test-utils';
import UpdateTab from './UpdateTab';

// Mock Tauri plugin-updater and plugin-process
vi.mock('@tauri-apps/plugin-updater', () => ({
  check: vi.fn(),
}));
vi.mock('@tauri-apps/plugin-process', () => ({
  relaunch: vi.fn(),
}));
vi.mock('@tauri-apps/api/app', () => ({
  getVersion: vi.fn().mockResolvedValue('1.2.3'),
}));

// Mock the hooks
vi.mock('../hooks/useAppUpdater', () => ({
  useAppUpdater: vi.fn(),
}));
vi.mock('../hooks/useMetadataSync', () => ({
  useMetadataSyncMutation: vi.fn(),
}));
vi.mock('../../../stores/useToastStore', () => ({
  useToastStore: () => ({
    addToast: vi.fn(),
  }),
}));

import { useAppUpdater } from '../hooks/useAppUpdater';
import { useMetadataSyncMutation } from '../hooks/useMetadataSync';

const makeUpdaterMock = (overrides = {}) => ({
  update: null,
  isChecking: false,
  isInstalling: false,
  progress: null,
  error: null,
  checkForUpdate: vi.fn(),
  downloadAndInstall: vi.fn(),
  dismiss: vi.fn(),
  ...overrides,
});

const makeMetaSyncMock = (overrides = {}) => ({
  isPending: false,
  mutate: vi.fn(),
  ...overrides,
});

describe('UpdateTab - TC-34', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('TC-34-001: No Update Available', () => {
    it('shows "latest version" message when no update and not checking', () => {
      vi.mocked(useAppUpdater).mockReturnValue(
        makeUpdaterMock() as ReturnType<typeof useAppUpdater>,
      );
      vi.mocked(useMetadataSyncMutation).mockReturnValue(
        makeMetaSyncMock() as unknown as unknown as ReturnType<typeof useMetadataSyncMutation>,
      );

      render(<UpdateTab />);

      expect(screen.getByText(/You are on the latest version/i)).toBeInTheDocument();
    });

    it('displays the current app version', async () => {
      vi.mocked(useAppUpdater).mockReturnValue(
        makeUpdaterMock() as ReturnType<typeof useAppUpdater>,
      );
      vi.mocked(useMetadataSyncMutation).mockReturnValue(
        makeMetaSyncMock() as unknown as unknown as ReturnType<typeof useMetadataSyncMutation>,
      );

      render(<UpdateTab />);

      await waitFor(() => {
        expect(screen.getByText(/v1\.2\.3/)).toBeInTheDocument();
      });
    });
  });

  describe('TC-34-002: Update Available UI', () => {
    it('shows update available alert when update is found', () => {
      vi.mocked(useAppUpdater).mockReturnValue(
        makeUpdaterMock({
          update: { version: '2.0.0', body: 'Bug fixes and improvements' },
        }) as ReturnType<typeof useAppUpdater>,
      );
      vi.mocked(useMetadataSyncMutation).mockReturnValue(
        makeMetaSyncMock() as unknown as unknown as ReturnType<typeof useMetadataSyncMutation>,
      );

      render(<UpdateTab />);

      expect(screen.getByText(/Update Available: v2\.0\.0/i)).toBeInTheDocument();
      expect(screen.getByText(/Bug fixes and improvements/i)).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /Install & Restart/i })).toBeInTheDocument();
    });

    it('hides "latest version" message when update is available', () => {
      vi.mocked(useAppUpdater).mockReturnValue(
        makeUpdaterMock({
          update: { version: '2.0.0', body: null },
        }) as ReturnType<typeof useAppUpdater>,
      );
      vi.mocked(useMetadataSyncMutation).mockReturnValue(
        makeMetaSyncMock() as unknown as unknown as ReturnType<typeof useMetadataSyncMutation>,
      );

      render(<UpdateTab />);

      expect(screen.queryByText(/You are on the latest version/i)).not.toBeInTheDocument();
    });
  });

  describe('TC-34-003: Check for Updates Button', () => {
    it('calls checkForUpdate when button is clicked', async () => {
      const mockCheck = vi.fn();
      vi.mocked(useAppUpdater).mockReturnValue(
        makeUpdaterMock({ checkForUpdate: mockCheck }) as ReturnType<typeof useAppUpdater>,
      );
      vi.mocked(useMetadataSyncMutation).mockReturnValue(
        makeMetaSyncMock() as unknown as unknown as ReturnType<typeof useMetadataSyncMutation>,
      );

      render(<UpdateTab />);

      fireEvent.click(screen.getByRole('button', { name: /Check for Updates/i }));

      await waitFor(() => {
        expect(mockCheck).toHaveBeenCalledTimes(1);
      });
    });

    it('shows "Checking..." and disables button while checking', () => {
      vi.mocked(useAppUpdater).mockReturnValue(
        makeUpdaterMock({ isChecking: true }) as ReturnType<typeof useAppUpdater>,
      );
      vi.mocked(useMetadataSyncMutation).mockReturnValue(
        makeMetaSyncMock() as unknown as unknown as ReturnType<typeof useMetadataSyncMutation>,
      );

      render(<UpdateTab />);

      const checkBtn = screen.getByRole('button', { name: /Checking/i });
      expect(checkBtn).toBeDisabled();
    });
  });

  describe('TC-34-004: Download Progress', () => {
    it('shows download progress bar with percentage', () => {
      vi.mocked(useAppUpdater).mockReturnValue(
        makeUpdaterMock({
          update: { version: '2.0.0', body: null },
          progress: { downloaded: 5 * 1024 * 1024, total: 10 * 1024 * 1024 },
          isInstalling: true,
        }) as ReturnType<typeof useAppUpdater>,
      );
      vi.mocked(useMetadataSyncMutation).mockReturnValue(
        makeMetaSyncMock() as unknown as unknown as ReturnType<typeof useMetadataSyncMutation>,
      );

      render(<UpdateTab />);

      expect(screen.getByText(/Downloading\.\.\./i)).toBeInTheDocument();
      expect(screen.getByText('50%')).toBeInTheDocument();
    });

    it('shows download progress without percentage when total is unknown', () => {
      vi.mocked(useAppUpdater).mockReturnValue(
        makeUpdaterMock({
          update: { version: '2.0.0', body: null },
          progress: { downloaded: 1024 * 1024, total: null },
        }) as ReturnType<typeof useAppUpdater>,
      );
      vi.mocked(useMetadataSyncMutation).mockReturnValue(
        makeMetaSyncMock() as unknown as unknown as ReturnType<typeof useMetadataSyncMutation>,
      );

      render(<UpdateTab />);

      expect(screen.getByText(/Downloading\.\.\./i)).toBeInTheDocument();
      expect(screen.queryByText(/%/)).not.toBeInTheDocument();
    });
  });

  describe('TC-34-005: Update Error Handling', () => {
    it('shows error message and Dismiss button on update error', () => {
      vi.mocked(useAppUpdater).mockReturnValue(
        makeUpdaterMock({
          error: 'Network connection failed',
        }) as ReturnType<typeof useAppUpdater>,
      );
      vi.mocked(useMetadataSyncMutation).mockReturnValue(
        makeMetaSyncMock() as unknown as unknown as ReturnType<typeof useMetadataSyncMutation>,
      );

      render(<UpdateTab />);

      expect(screen.getByText(/Network connection failed/i)).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /Dismiss/i })).toBeInTheDocument();
    });

    it('calls dismiss when Dismiss button is clicked', () => {
      const mockDismiss = vi.fn();
      vi.mocked(useAppUpdater).mockReturnValue(
        makeUpdaterMock({
          error: 'Some error',
          dismiss: mockDismiss,
        }) as ReturnType<typeof useAppUpdater>,
      );
      vi.mocked(useMetadataSyncMutation).mockReturnValue(
        makeMetaSyncMock() as unknown as ReturnType<typeof useMetadataSyncMutation>,
      );

      render(<UpdateTab />);

      fireEvent.click(screen.getByRole('button', { name: /Dismiss/i }));
      expect(mockDismiss).toHaveBeenCalledTimes(1);
    });
  });

  describe('TC-34-006: Metadata Sync', () => {
    it('renders Metadata Sync section with Sync Now button', () => {
      vi.mocked(useAppUpdater).mockReturnValue(
        makeUpdaterMock() as ReturnType<typeof useAppUpdater>,
      );
      vi.mocked(useMetadataSyncMutation).mockReturnValue(
        makeMetaSyncMock() as unknown as unknown as ReturnType<typeof useMetadataSyncMutation>,
      );

      render(<UpdateTab />);

      expect(screen.getByText(/Metadata Sync/i)).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /Sync Now/i })).toBeInTheDocument();
    });

    it('calls metadata mutate when Sync Now clicked', () => {
      const mockMutate = vi.fn();
      vi.mocked(useAppUpdater).mockReturnValue(
        makeUpdaterMock() as ReturnType<typeof useAppUpdater>,
      );
      vi.mocked(useMetadataSyncMutation).mockReturnValue(
        makeMetaSyncMock({ mutate: mockMutate }) as unknown as ReturnType<
          typeof useMetadataSyncMutation
        >,
      );

      render(<UpdateTab />);

      fireEvent.click(screen.getByRole('button', { name: /Sync Now/i }));
      expect(mockMutate).toHaveBeenCalledTimes(1);
    });

    it('shows "Syncing..." and disables Sync Now button while pending', () => {
      vi.mocked(useAppUpdater).mockReturnValue(
        makeUpdaterMock() as ReturnType<typeof useAppUpdater>,
      );
      vi.mocked(useMetadataSyncMutation).mockReturnValue(
        makeMetaSyncMock({ isPending: true }) as unknown as ReturnType<
          typeof useMetadataSyncMutation
        >,
      );

      render(<UpdateTab />);

      const syncBtn = screen.getByRole('button', { name: /Syncing\.\.\./i });
      expect(syncBtn).toBeDisabled();
    });
  });
});
