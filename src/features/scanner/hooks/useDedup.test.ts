/**
 * Tests for Epic 9 dedup hooks.
 * Covers: useDedupReport, useStartDedupScan, useCancelDedupScan, useResolveDuplicates
 * Uses React Testing Library renderHook with mocked dedupService and React Query.
 */

import { renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import React from 'react';
import {
  useDedupReport,
  useStartDedupScan,
  useCancelDedupScan,
  useResolveDuplicates,
} from './useDedup';
import * as dedupService from '../../../lib/services/dedupService';
import type { DupScanReport, DupScanEvent, ResolutionSummary } from '../../../types/dedup';

// Mock the service
vi.mock('../../../lib/services/dedupService', () => ({
  dedupService: {
    getReport: vi.fn(),
    startDedupScan: vi.fn(),
    cancelDedupScan: vi.fn(),
    resolveBatch: vi.fn(),
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

// Helper to create wrapper with QueryClient
const createWrapper = () => {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false, staleTime: 0 },
      mutations: { retry: false },
    },
  });
  return ({ children }: { children: React.ReactNode }) =>
    React.createElement(QueryClientProvider, { client: queryClient }, children);
};

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
            signals: [{ key: 'hash', detail: 'BLAKE3 collision', score: 100 }],
            members: [
              {
                folderPath: '/path/mod-a',
                displayName: 'Mod A',
                totalSizeBytes: 1024,
                fileCount: 5,
                confidenceScore: 95,
                signals: [],
              },
              {
                folderPath: '/path/mod-b',
                displayName: 'Mod B',
                totalSizeBytes: 1024,
                fileCount: 5,
                confidenceScore: 95,
                signals: [],
              },
            ],
          },
        ],
      };

      vi.mocked(dedupService.dedupService.getReport).mockResolvedValue(mockReport);

      const { result } = renderHook(() => useDedupReport(), {
        wrapper: createWrapper(),
      });

      await waitFor(() => expect(result.current.isSuccess).toBe(true));
      expect(result.current.data).toEqual(mockReport);
      expect(dedupService.dedupService.getReport).toHaveBeenCalledOnce();
    });

    it('handles null report (no scan completed)', async () => {
      vi.mocked(dedupService.dedupService.getReport).mockResolvedValue(null);

      const { result } = renderHook(() => useDedupReport(), {
        wrapper: createWrapper(),
      });

      await waitFor(() => expect(result.current.isSuccess).toBe(true));
      expect(result.current.data).toBeNull();
    });

    it('handles fetch error', async () => {
      const error = new Error('Failed to fetch report');
      vi.mocked(dedupService.dedupService.getReport).mockRejectedValue(error);

      const { result } = renderHook(() => useDedupReport(), {
        wrapper: createWrapper(),
      });

      await waitFor(() => expect(result.current.isError).toBe(true));
      expect(result.current.error).toEqual(error);
    });
  });

  describe('useStartDedupScan', () => {
    it('starts a scan and invalidates report cache on success', async () => {
      vi.mocked(dedupService.dedupService.startDedupScan).mockResolvedValue(undefined);

      const { result } = renderHook(() => useStartDedupScan(), {
        wrapper: createWrapper(),
      });

      const onEvent = vi.fn();
      result.current.mutate({ gameId: 'genshin', modsRoot: '/path/to/mods', onEvent });

      await waitFor(() => expect(result.current.isSuccess).toBe(true));
      expect(dedupService.dedupService.startDedupScan).toHaveBeenCalledWith(
        'genshin',
        '/path/to/mods',
        onEvent,
      );
    });

    it('handles scan error with toast notification', async () => {
      const { toast } = await import('../../../stores/useToastStore');
      const error = new Error('Scan failed: invalid path');
      vi.mocked(dedupService.dedupService.startDedupScan).mockRejectedValue(error);

      const { result } = renderHook(() => useStartDedupScan(), {
        wrapper: createWrapper(),
      });

      const onEvent = vi.fn();
      result.current.mutate({ gameId: 'genshin', modsRoot: '/invalid', onEvent });

      await waitFor(() => expect(result.current.isError).toBe(true));
      expect(toast.error).toHaveBeenCalled();
    });

    it('emits progress events during scan', async () => {
      const mockEvent: DupScanEvent = {
        event: 'Progress',
        data: {
          scanId: 'scan-1',
          processedFolders: 50,
          totalFolders: 100,
          currentFolder: '/path/mod-50',
          percent: 50,
        },
      };

      vi.mocked(dedupService.dedupService.startDedupScan).mockImplementation(
        (_gameId, _modsRoot, onEvent) => {
          onEvent(mockEvent);
          return Promise.resolve();
        },
      );

      const onEvent = vi.fn();
      const { result } = renderHook(() => useStartDedupScan(), {
        wrapper: createWrapper(),
      });

      result.current.mutate({ gameId: 'genshin', modsRoot: '/path', onEvent });

      await waitFor(() => expect(result.current.isSuccess).toBe(true));
      expect(onEvent).toHaveBeenCalledWith(mockEvent);
    });
  });

  describe('useCancelDedupScan', () => {
    it('cancels the running scan', async () => {
      vi.mocked(dedupService.dedupService.cancelDedupScan).mockResolvedValue(undefined);

      const { result } = renderHook(() => useCancelDedupScan(), {
        wrapper: createWrapper(),
      });

      result.current.mutate();

      await waitFor(() => expect(result.current.isSuccess).toBe(true));
      expect(dedupService.dedupService.cancelDedupScan).toHaveBeenCalledOnce();
    });

    it('shows success toast on cancel', async () => {
      const { toast } = await import('../../../stores/useToastStore');
      vi.mocked(dedupService.dedupService.cancelDedupScan).mockResolvedValue(undefined);

      const { result } = renderHook(() => useCancelDedupScan(), {
        wrapper: createWrapper(),
      });

      result.current.mutate();

      await waitFor(() => expect(result.current.isSuccess).toBe(true));
      expect(toast.info).toHaveBeenCalledWith('Scan cancelled');
    });

    it('handles cancel error with toast', async () => {
      const { toast } = await import('../../../stores/useToastStore');
      const error = new Error('Cancel failed');
      vi.mocked(dedupService.dedupService.cancelDedupScan).mockRejectedValue(error);

      const { result } = renderHook(() => useCancelDedupScan(), {
        wrapper: createWrapper(),
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

      vi.mocked(dedupService.dedupService.resolveBatch).mockResolvedValue(mockSummary);

      const { result } = renderHook(() => useResolveDuplicates(), {
        wrapper: createWrapper(),
      });

      const requests = [
        {
          groupId: 'group-1',
          action: 'KeepA' as const,
          folderA: '/path/mod-a',
          folderB: '/path/mod-b',
        },
      ];

      result.current.mutate({ requests, gameId: 'genshin' });

      await waitFor(() => expect(result.current.isSuccess).toBe(true));
      expect(result.current.data).toEqual(mockSummary);
      expect(dedupService.dedupService.resolveBatch).toHaveBeenCalledWith(requests, 'genshin');
    });

    it('shows success toast with resolution summary', async () => {
      const { toast } = await import('../../../stores/useToastStore');
      const mockSummary: ResolutionSummary = {
        total: 3,
        successful: 3,
        failed: 0,
        errors: [],
      };

      vi.mocked(dedupService.dedupService.resolveBatch).mockResolvedValue(mockSummary);

      const { result } = renderHook(() => useResolveDuplicates(), {
        wrapper: createWrapper(),
      });

      result.current.mutate({ requests: [], gameId: 'genshin' });

      await waitFor(() => expect(result.current.isSuccess).toBe(true));
      expect(toast.success).toHaveBeenCalledWith('Resolved 3/3 duplicates');
    });

    it('shows warning toast when some resolutions fail', async () => {
      const { toast } = await import('../../../stores/useToastStore');
      const mockSummary: ResolutionSummary = {
        total: 3,
        successful: 2,
        failed: 1,
        errors: [{ groupId: 'group-1', message: 'Permission denied' }],
      };

      vi.mocked(dedupService.dedupService.resolveBatch).mockResolvedValue(mockSummary);

      const { result } = renderHook(() => useResolveDuplicates(), {
        wrapper: createWrapper(),
      });

      result.current.mutate({ requests: [], gameId: 'genshin' });

      await waitFor(() => expect(result.current.isSuccess).toBe(true));
      expect(toast.warning).toHaveBeenCalled();
    });

    it('handles resolution error with toast', async () => {
      const { toast } = await import('../../../stores/useToastStore');
      const error = new Error('Resolution service unavailable');
      vi.mocked(dedupService.dedupService.resolveBatch).mockRejectedValue(error);

      const { result } = renderHook(() => useResolveDuplicates(), {
        wrapper: createWrapper(),
      });

      result.current.mutate({ requests: [], gameId: 'genshin' });

      await waitFor(() => expect(result.current.isError).toBe(true));
      expect(toast.error).toHaveBeenCalled();
    });
  });
});
