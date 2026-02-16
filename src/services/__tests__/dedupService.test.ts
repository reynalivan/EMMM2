/* eslint-disable @typescript-eslint/no-explicit-any */
/**
 * Tests for Epic 9 dedup service.
 * Covers: startDedupScan, cancelDedupScan, getReport, resolveBatch
 * Mocks Tauri invoke and Channel APIs.
 */

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { dedupService } from '../dedupService';
import type { DupScanReport, DupScanEvent, ResolutionSummary } from '../../types/dedup';

// Mock Tauri API
let channelInstances: Array<{ onmessage: ((event: any) => void) | null }> = [];

vi.mock('@tauri-apps/api/core', () => {
  class MockChannel {
    onmessage: ((event: any) => void) | null = null;
  }
  return {
    invoke: vi.fn(),
    Channel: vi.fn(function (this: any) {
      const instance = new MockChannel();
      channelInstances.push(instance);
      return instance;
    }),
  };
});

describe('dedupService', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    channelInstances = [];
  });

  describe('startDedupScan', () => {
    it('creates a Channel and starts scan with invoke', async () => {
      const onEvent = vi.fn();

      vi.mocked(invoke).mockResolvedValue(undefined);

      await dedupService.startDedupScan('genshin', '/path/to/mods', onEvent);

      expect(invoke).toHaveBeenCalledWith('dup_scan_start', {
        gameId: 'genshin',
        modsRoot: '/path/to/mods',
        onEvent: expect.any(Object),
      });
    });

    it('sets up onmessage callback for streaming events', async () => {
      const onEvent = vi.fn();

      vi.mocked(invoke).mockResolvedValue(undefined);

      await dedupService.startDedupScan('genshin', '/path/to/mods', onEvent);

      // Simulate receiving a progress event
      const mockEvent: DupScanEvent = {
        event: 'Progress',
        data: {
          scanId: 'scan-1',
          processedFolders: 10,
          totalFolders: 100,
          currentFolder: '/path/mod-10',
          percent: 10,
        },
      };

      // Get the channel instance that was created
      const channel = channelInstances[0];
      if (channel && channel.onmessage) {
        channel.onmessage(mockEvent);
      }

      expect(onEvent).toHaveBeenCalledWith(mockEvent);
    });

    it('handles scan errors during invoke', async () => {
      const onEvent = vi.fn();
      const error = new Error('Invalid path');

      vi.mocked(invoke).mockRejectedValue(error);

      await expect(dedupService.startDedupScan('genshin', '/invalid', onEvent)).rejects.toThrow(
        'Invalid path',
      );
    });
  });

  describe('cancelDedupScan', () => {
    it('invokes dup_scan_cancel command', async () => {
      vi.mocked(invoke).mockResolvedValue(undefined);

      await dedupService.cancelDedupScan();

      expect(invoke).toHaveBeenCalledWith('dup_scan_cancel');
    });

    it('handles cancel errors gracefully', async () => {
      const error = new Error('No active scan');

      vi.mocked(invoke).mockRejectedValue(error);

      await expect(dedupService.cancelDedupScan()).rejects.toThrow('No active scan');
    });
  });

  describe('getReport', () => {
    it('fetches the last completed scan report', async () => {
      const mockReport: DupScanReport = {
        scanId: 'scan-1',
        gameId: 'genshin',
        rootPath: '/path/to/mods',
        totalGroups: 1,
        totalMembers: 2,
        groups: [
          {
            groupId: 'group-1',
            confidenceScore: 95,
            matchReason: 'Hash match',
            signals: [{ key: 'hash', detail: 'BLAKE3', score: 100 }],
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

      vi.mocked(invoke).mockResolvedValue(mockReport);

      const result = await dedupService.getReport();

      expect(invoke).toHaveBeenCalledWith('dup_scan_get_report');
      expect(result).toEqual(mockReport);
    });

    it('returns null if no report exists', async () => {
      vi.mocked(invoke).mockResolvedValue(null);

      const result = await dedupService.getReport();

      expect(result).toBeNull();
    });

    it('handles report fetch errors', async () => {
      const error = new Error('Database error');

      vi.mocked(invoke).mockRejectedValue(error);

      await expect(dedupService.getReport()).rejects.toThrow('Database error');
    });
  });

  describe('resolveBatch', () => {
    it('sends resolution requests and returns summary', async () => {
      const mockSummary: ResolutionSummary = {
        total: 2,
        successful: 2,
        failed: 0,
        errors: [],
      };

      vi.mocked(invoke).mockResolvedValue(mockSummary);

      const requests = [
        {
          groupId: 'group-1',
          action: 'KeepA' as const,
          folderA: '/path/mod-a',
          folderB: '/path/mod-b',
        },
        {
          groupId: 'group-2',
          action: 'Ignore' as const,
          folderA: '/path/mod-c',
          folderB: '/path/mod-d',
        },
      ];

      const result = await dedupService.resolveBatch(requests, 'genshin');

      expect(invoke).toHaveBeenCalledWith('dup_resolve_batch', {
        requests,
        gameId: 'genshin',
      });
      expect(result).toEqual(mockSummary);
    });

    it('handles partial resolution failures', async () => {
      const mockSummary: ResolutionSummary = {
        total: 3,
        successful: 2,
        failed: 1,
        errors: [
          {
            groupId: 'group-2',
            message: 'Permission denied when deleting folder',
          },
        ],
      };

      vi.mocked(invoke).mockResolvedValue(mockSummary);

      const requests = [
        {
          groupId: 'group-1',
          action: 'KeepA' as const,
          folderA: '/path/mod-a',
          folderB: '/path/mod-b',
        },
      ];

      const result = await dedupService.resolveBatch(requests, 'genshin');

      expect(result.failed).toBe(1);
      expect(result.errors).toHaveLength(1);
    });

    it('handles resolution errors', async () => {
      const error = new Error('Backend service error');

      vi.mocked(invoke).mockRejectedValue(error);

      const requests = [
        {
          groupId: 'group-1',
          action: 'KeepA' as const,
          folderA: '/path/mod-a',
          folderB: '/path/mod-b',
        },
      ];

      await expect(dedupService.resolveBatch(requests, 'genshin')).rejects.toThrow(
        'Backend service error',
      );
    });
  });
});
