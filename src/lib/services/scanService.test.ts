import { describe, it, expect, vi, beforeEach } from 'vitest';
import { scanService } from './scanService';
import { invoke, Channel } from '@tauri-apps/api/core';

vi.mock('@tauri-apps/api/core', () => {
  class ChannelMock {
    onmessage: unknown = null;
  }
  return {
    invoke: vi.fn(),
    Channel: ChannelMock,
  };
});

describe('scanService', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('getMasterDb', () => {
    it('should invoke get_master_db with gameType', async () => {
      vi.mocked(invoke).mockResolvedValueOnce('{"mock":"json"}');
      const result = await scanService.getMasterDb('GIMI');
      expect(invoke).toHaveBeenCalledWith('get_master_db', { gameType: 'GIMI' });
      expect(result).toBe('{"mock":"json"}');
    });
  });

  describe('detectArchives', () => {
    it('should invoke detect_archives_cmd', async () => {
      await scanService.detectArchives('/mods/path');
      expect(invoke).toHaveBeenCalledWith('detect_archives_cmd', { modsPath: '/mods/path' });
    });
  });

  describe('extractArchive', () => {
    it('should invoke extract_archive_cmd with correct params', async () => {
      await scanService.extractArchive('/archive.zip', '/mods', 'pass123', true);
      expect(invoke).toHaveBeenCalledWith('extract_archive_cmd', {
        archivePath: '/archive.zip',
        modsDir: '/mods',
        password: 'pass123',
        overwrite: true,
      });
    });

    it('should pass null for undefined password', async () => {
      await scanService.extractArchive('/archive.zip', '/mods', undefined, false);
      expect(invoke).toHaveBeenCalledWith('extract_archive_cmd', {
        archivePath: '/archive.zip',
        modsDir: '/mods',
        password: null,
        overwrite: false,
      });
    });
  });

  describe('analyzeArchive', () => {
    it('should invoke analyze_archive_cmd', async () => {
      await scanService.analyzeArchive('/archive.zip');
      expect(invoke).toHaveBeenCalledWith('analyze_archive_cmd', { archivePath: '/archive.zip' });
    });
  });

  describe('startScan', () => {
    it('should fetch master db and invoke start_scan with channel', async () => {
      vi.mocked(invoke).mockImplementation(async (cmd) => {
        if (cmd === 'get_master_db') return '[]';
        return [{ folderPath: '/test' }];
      });

      const onEvent = vi.fn();
      const result = await scanService.startScan('GIMI', '/mods', onEvent);

      expect(invoke).toHaveBeenCalledWith('get_master_db', { gameType: 'GIMI' });

      const startScanCallArgs = vi
        .mocked(invoke)
        .mock.calls.find((c) => c[0] === 'start_scan')?.[1] as Record<string, unknown>;
      expect(startScanCallArgs).toBeDefined();
      expect(startScanCallArgs.modsPath).toBe('/mods');
      expect(startScanCallArgs.dbJson).toBe('[]');
      expect(startScanCallArgs.onProgress).toBeInstanceOf(Channel);

      expect(result).toEqual([{ folderPath: '/test' }]);

      // Test channel message routing
      const channelInstance =
        startScanCallArgs.onProgress as import('@tauri-apps/api/core').Channel<{
          type: string;
          message?: string;
        }>;
      expect(channelInstance.onmessage).toBeInstanceOf(Function);

      const mockEvent = { type: 'progress', message: 'test' };
      channelInstance.onmessage(mockEvent);
      expect(onEvent).toHaveBeenCalledWith(mockEvent);
    });
  });

  describe('getScanResult', () => {
    it('should fetch master db and invoke get_scan_result', async () => {
      vi.mocked(invoke).mockImplementation(async (cmd) => {
        if (cmd === 'get_master_db') return '[]';
        return [];
      });

      await scanService.getScanResult('GIMI', '/mods');
      expect(invoke).toHaveBeenCalledWith('get_scan_result', {
        modsPath: '/mods',
        dbJson: '[]',
      });
    });
  });

  describe('detectConflicts', () => {
    it('should invoke detect_conflicts_cmd', async () => {
      await scanService.detectConflicts(['/path1', '/path2']);
      expect(invoke).toHaveBeenCalledWith('detect_conflicts_cmd', {
        iniPaths: ['/path1', '/path2'],
      });
    });
  });

  describe('detectConflictsInFolder', () => {
    it('should invoke detect_conflicts_in_folder_cmd', async () => {
      await scanService.detectConflictsInFolder('/mods');
      expect(invoke).toHaveBeenCalledWith('detect_conflicts_in_folder_cmd', { modsPath: '/mods' });
    });
  });

  describe('cancelScan', () => {
    it('should invoke cancel_scan_cmd', async () => {
      await scanService.cancelScan();
      expect(invoke).toHaveBeenCalledWith('cancel_scan_cmd');
    });
  });

  describe('syncDatabase', () => {
    it('should fetch master db and invoke sync_database_cmd', async () => {
      vi.mocked(invoke).mockImplementation(async (cmd) => {
        if (cmd === 'get_master_db') return '[]';
        return { total_scanned: 10 };
      });

      const onEvent = vi.fn();
      const result = await scanService.syncDatabase('g1', 'Genshin', 'GIMI', '/mods', onEvent);

      const syncCallArgs = vi
        .mocked(invoke)
        .mock.calls.find((c) => c[0] === 'sync_database_cmd')?.[1] as Record<string, unknown>;
      expect(syncCallArgs.gameId).toBe('g1');
      expect(syncCallArgs.dbJson).toBe('[]');
      expect(syncCallArgs.onProgress).toBeInstanceOf(Channel);
      expect(result).toEqual({ total_scanned: 10 });

      const channel = syncCallArgs.onProgress as import('@tauri-apps/api/core').Channel<{
        type: string;
      }>;
      channel.onmessage({ type: 'done' });
      expect(onEvent).toHaveBeenCalledWith({ type: 'done' });
    });

    it('should work without onEvent', async () => {
      vi.mocked(invoke).mockImplementation(async (cmd) => {
        if (cmd === 'get_master_db') return '[]';
        return { total_scanned: 10 };
      });

      await scanService.syncDatabase('g1', 'Genshin', 'GIMI', '/mods');

      const syncCallArgs = vi
        .mocked(invoke)
        .mock.calls.find((c) => c[0] === 'sync_database_cmd')?.[1] as Record<string, unknown>;
      expect(syncCallArgs.onProgress).toBeInstanceOf(Channel);
    });
  });

  describe('scanPreview', () => {
    it('should fetch master db and invoke scan_preview_cmd', async () => {
      vi.mocked(invoke).mockImplementation(async (cmd) => {
        if (cmd === 'get_master_db') return '[]';
        return [];
      });

      const onEvent = vi.fn();
      await scanService.scanPreview('g1', 'GIMI', '/mods', onEvent, ['/specific']);

      const previewCallArgs = vi
        .mocked(invoke)
        .mock.calls.find((c) => c[0] === 'scan_preview_cmd')?.[1] as Record<string, unknown>;
      expect(previewCallArgs.gameId).toBe('g1');
      expect(previewCallArgs.specificPaths).toEqual(['/specific']);
      expect(previewCallArgs.onProgress).toBeInstanceOf(Channel);

      const channel = previewCallArgs.onProgress as import('@tauri-apps/api/core').Channel<{
        type: string;
      }>;
      channel.onmessage({ type: 'progress' });
      expect(onEvent).toHaveBeenCalledWith({ type: 'progress' });
    });
  });

  describe('quickImport', () => {
    it('should invoke sync_database_cmd with empty dbJson', async () => {
      await scanService.quickImport('g1', 'Genshin', 'GIMI', '/mods');

      const args = vi
        .mocked(invoke)
        .mock.calls.find((c) => c[0] === 'sync_database_cmd')?.[1] as Record<string, unknown>;
      expect(args.dbJson).toBe('[]');
    });
  });

  describe('commitScan', () => {
    it('should invoke commit_scan_cmd', async () => {
      const items = [
        { folderPath: '/a', skip: false } as unknown as Parameters<
          typeof scanService.commitScan
        >[4][0],
      ];
      await scanService.commitScan('g1', 'Genshin', 'GIMI', '/mods', items);
      expect(invoke).toHaveBeenCalledWith('commit_scan_cmd', {
        gameId: 'g1',
        gameName: 'Genshin',
        gameType: 'GIMI',
        modsPath: '/mods',
        items,
      });
    });
  });

  describe('scoreCandidatesBatch', () => {
    it('should fetch master db and invoke score_candidates_batch_cmd', async () => {
      vi.mocked(invoke).mockImplementation(async (cmd) => {
        if (cmd === 'get_master_db') return '[]';
        return { Candidate1: 85 };
      });

      const result = await scanService.scoreCandidatesBatch('/mods/MyMod', ['Candidate1'], 'GIMI');

      expect(invoke).toHaveBeenCalledWith('score_candidates_batch_cmd', {
        folderPath: '/mods/MyMod',
        candidateNames: ['Candidate1'],
        dbJson: '[]',
      });
      expect(result).toEqual({ Candidate1: 85 });
    });
  });
});
