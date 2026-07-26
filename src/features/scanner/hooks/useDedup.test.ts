/**
 * Tests for Epic 9 dedup hooks.
 * Covers: useDedupReport, useStartDedupScan, useCancelDedupScan, useResolveDuplicates
 * Uses React Testing Library renderHook with mocked IPC commands and React Query.
 */

import { renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
  useDedupReport,
  useStartDedupScan,
  useCancelDedupScan,
  useResolveDuplicates,
} from './useDedup';
import { commands } from '../../../lib/bindings';
import type { DupScanReport, DupScanEvent, ResolutionSummary } from '../../../types/scanner';
import { createWrapper } from '../../../testing/test-utils';

vi.unmock('@tanstack/react-query');

// The hook constructs a Channel, so the local core mock must provide one.
vi.mock('@tauri-apps/api/core', () => {
  class ChannelMock {
    onmessage: ((message: unknown) => void) | null = null;
  }
  return { invoke: vi.fn(), Channel: ChannelMock };
});

vi.mock('../../../lib/bindings', () => ({
  sparse: (value: unknown) => value,
  commands: {
    dupScanGetReport: vi.fn(),
    dupScanStart: vi.fn(),
    dupScanCancel: vi.fn(),
    dupResolveBatch: vi.fn(),
  },
}));

// Mock toast store
vi.mock('../../../stores/useToastStore', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
    warning: vi.fn(),
  },
}));

describe('useDedup hooks', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('useDedupReport', () => {
    it('fetches duplicate scan report', async () => {
      const mockReport: DupScanReport = {
        scanId: 'scan-1',
        gameId: 'genshin',
        rootPath: '/path/to/mods',
        totalGroups: 2,
        totalMembers: 4,
        groups: [
          {
            groupId: 'group-1',
            confidenceScore: 95,
            matchReason: 'Hash match',
            isUnsafe: false,
            signals: [{ key: 'hash', detail: 'BLAKE3 collision', score: 100 }],
            members: [
              {
                folderPath: '/path/mod-a',
                displayName: 'Mod A',
                totalSizeBytes: 1024,
                fileCount: 5,
                isSafe: true,
                confidenceScore: 95,
                signals: [],
                modId: null,
                version: 1,
              },
              {
                folderPath: '/path/mod-b',
                displayName: 'Mod B',
                totalSizeBytes: 1024,
                fileCount: 5,
                isSafe: true,
                confidenceScore: 95,
                signals: [],
                modId: null,
                version: 1,
              },
            ],
          },
        ],
      };

      vi.mocked(commands.dupScanGetReport).mockResolvedValue(mockReport);

      const { result } = renderHook(() => useDedupReport(), {
        wrapper: createWrapper,
      });

      await waitFor(() => expect(result.current.isSuccess).toBe(true));
      expect(result.current.data).toEqual(mockReport);
      expect(commands.dupScanGetReport).toHaveBeenCalledOnce();
    });

    it('handles null report (no scan completed)', async () => {
      vi.mocked(commands.dupScanGetReport).mockResolvedValue(null);

      const { result } = renderHook(() => useDedupReport(), {
        wrapper: createWrapper,
      });

      await waitFor(() => expect(result.current.isSuccess).toBe(true));
      expect(result.current.data).toBeNull();
    });

    it('handles fetch error', async () => {
      const error = new Error('Failed to fetch report');
      vi.mocked(commands.dupScanGetReport).mockRejectedValue(error);

      const { result } = renderHook(() => useDedupReport(), {
        wrapper: createWrapper,
      });

      await waitFor(() => expect(result.current.isError).toBe(true));
      expect(result.current.error).toEqual(error);
    });
  });

  describe('useStartDedupScan', () => {
    it('starts a scan and invalidates report cache on success', async () => {
      vi.mocked(commands.dupScanStart).mockResolvedValue(undefined);

      const { result } = renderHook(() => useStartDedupScan(), {
        wrapper: createWrapper,
      });

      const onEvent = vi.fn();
      result.current.mutate({ gameId: 'genshin', modsRoot: '/path/to/mods', onEvent });

      await waitFor(() => expect(result.current.isSuccess).toBe(true));
      expect(commands.dupScanStart).toHaveBeenCalledWith(
        'genshin',
        '/path/to/mods',
        expect.anything(),
      );
    });

    it('handles scan error with toast notification', async () => {
      const { toast } = await import('../../../stores/useToastStore');
      const error = new Error('Scan failed: invalid path');
      vi.mocked(commands.dupScanStart).mockRejectedValue(error);

      const { result } = renderHook(() => useStartDedupScan(), {
        wrapper: createWrapper,
      });

      const onEvent = vi.fn();
      result.current.mutate({ gameId: 'genshin', modsRoot: '/invalid', onEvent });

      await waitFor(() => expect(result.current.isError).toBe(true));
      expect(toast.error).toHaveBeenCalled();
    });

    it('emits progress events during scan', async () => {
      const mockEvent: DupScanEvent = {
        event: 'progress',
        data: {
          scanId: 'scan-1',
          processedFolders: 50,
          totalFolders: 100,
          currentFolder: '/path/mod-50',
          percent: 50,
        },
      };

      vi.mocked(commands.dupScanStart).mockImplementation((_gameId, _modsRoot, channel) => {
        channel.onmessage?.(mockEvent);
        return Promise.resolve();
      });

      const onEvent = vi.fn();
      const { result } = renderHook(() => useStartDedupScan(), {
        wrapper: createWrapper,
      });

      result.current.mutate({ gameId: 'genshin', modsRoot: '/path', onEvent });

      await waitFor(() => expect(result.current.isSuccess).toBe(true));
      expect(onEvent).toHaveBeenCalledWith(mockEvent);
    });
  });

  describe('useCancelDedupScan', () => {
    it('cancels the running scan', async () => {
      vi.mocked(commands.dupScanCancel).mockResolvedValue(undefined);

      const { result } = renderHook(() => useCancelDedupScan(), {
        wrapper: createWrapper,
      });

      result.current.mutate();

      await waitFor(() => expect(result.current.isSuccess).toBe(true));
      expect(commands.dupScanCancel).toHaveBeenCalledOnce();
    });

    it('shows success toast on cancel', async () => {
      const { toast } = await import('../../../stores/useToastStore');
      vi.mocked(commands.dupScanCancel).mockResolvedValue(undefined);

      const { result } = renderHook(() => useCancelDedupScan(), {
        wrapper: createWrapper,
      });

      result.current.mutate();

      await waitFor(() => expect(result.current.isSuccess).toBe(true));
      expect(toast.info).toHaveBeenCalled();
    });

    it('handles cancel error with toast', async () => {
      const { toast } = await import('../../../stores/useToastStore');
      const error = new Error('Cancel failed');
      vi.mocked(commands.dupScanCancel).mockRejectedValue(error);

      const { result } = renderHook(() => useCancelDedupScan(), {
        wrapper: createWrapper,
      });

      result.current.mutate();

      await waitFor(() => expect(result.current.isError).toBe(true));
      expect(toast.error).toHaveBeenCalled();
    });
  });

  describe('useResolveDuplicates', () => {
    it('resolves duplicates with batch requests', async () => {
      const mockSummary: ResolutionSummary = {
        total: 2,
        successful: 2,
        failed: 0,
        errors: [],
      };

      vi.mocked(commands.dupResolveBatch).mockResolvedValue(mockSummary);

      const { result } = renderHook(() => useResolveDuplicates(), {
        wrapper: createWrapper,
      });

      const requests = [
        {
          groupId: 'group-1',
          action: 'keepA' as const,
          folderA: '/path/mod-a',
          folderB: '/path/mod-b',
        },
      ];

      result.current.mutate({ requests, gameId: 'genshin' });

      await waitFor(() => expect(result.current.isSuccess).toBe(true));
      expect(result.current.data).toEqual(mockSummary);
      expect(commands.dupResolveBatch).toHaveBeenCalledWith(requests, 'genshin');
    });

    it('shows success toast with resolution summary', async () => {
      const { toast } = await import('../../../stores/useToastStore');
      const mockSummary: ResolutionSummary = {
        total: 3,
        successful: 3,
        failed: 0,
        errors: [],
      };

      vi.mocked(commands.dupResolveBatch).mockResolvedValue(mockSummary);

      const { result } = renderHook(() => useResolveDuplicates(), {
        wrapper: createWrapper,
      });

      result.current.mutate({ requests: [], gameId: 'genshin' });

      await waitFor(() => expect(result.current.isSuccess).toBe(true));
      expect(toast.success).toHaveBeenCalled();
    });

    it('shows warning toast when some resolutions fail', async () => {
      const { toast } = await import('../../../stores/useToastStore');
      const mockSummary: ResolutionSummary = {
        total: 3,
        successful: 2,
        failed: 1,
        errors: [{ groupId: 'group-1', message: '', action: 'ignore' }],
      };

      vi.mocked(commands.dupResolveBatch).mockResolvedValue(mockSummary);

      const { result } = renderHook(() => useResolveDuplicates(), {
        wrapper: createWrapper,
      });

      result.current.mutate({ requests: [], gameId: 'genshin' });

      await waitFor(() => expect(result.current.isSuccess).toBe(true));
      expect(toast.warning).toHaveBeenCalled();
    });

    it('handles resolution error with toast', async () => {
      const { toast } = await import('../../../stores/useToastStore');
      const error = new Error('Resolution service unavailable');
      vi.mocked(commands.dupResolveBatch).mockRejectedValue(error);

      const { result } = renderHook(() => useResolveDuplicates(), {
        wrapper: createWrapper,
      });

      result.current.mutate({ requests: [], gameId: 'genshin' });

      await waitFor(() => expect(result.current.isError).toBe(true));
      expect(toast.error).toHaveBeenCalled();
    });
  });
});
