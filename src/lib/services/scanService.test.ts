import { describe, it, expect, vi, beforeEach } from 'vitest';
import { scanService } from './scanService';
import { invoke, Channel } from '@tauri-apps/api/core';
import { GameType } from '../../types/game';

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
      const result = await scanService.getMasterDb(GameType.GIMI);
      expect(invoke).toHaveBeenCalledWith('get_master_db', { gameType: GameType.GIMI });
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
        customName: null,
        disableAfter: false,
        unpackNested: true,
        onProgress: expect.anything(),
      });
    });

    it('should pass null for undefined password', async () => {
      await scanService.extractArchive('/archive.zip', '/mods', undefined, false);
      expect(invoke).toHaveBeenCalledWith('extract_archive_cmd', {
        archivePath: '/archive.zip',
        modsDir: '/mods',
        password: null,
        overwrite: false,
        customName: null,
        disableAfter: false,
        unpackNested: true,
        onProgress: expect.anything(),
      });
    });
  });

  describe('analyzeArchive', () => {
    it('should invoke analyze_archive_cmd', async () => {
      await scanService.analyzeArchive('/archive.zip');
      expect(invoke).toHaveBeenCalledWith('analyze_archive_cmd', { archivePath: '/archive.zip' });
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

  describe('runDeepmatchScanner', () => {
    it('should fetch master db and invoke deepmatch_scanner_cmd', async () => {
      vi.mocked(invoke).mockImplementation(async (cmd) => {
        if (cmd === 'get_master_db') return '[]';
        return { total_scanned: 10 };
      });

      const onEvent = vi.fn();
      const result = await scanService.runDeepmatchScanner(
        'g1',
        'Genshin',
        GameType.GIMI,
        '/mods',
        onEvent,
      );

      const syncCallArgs = vi
        .mocked(invoke)
        .mock.calls.find((c) => c[0] === 'deepmatch_scanner_cmd')?.[1] as Record<string, unknown>;
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

      await scanService.runDeepmatchScanner('g1', 'Genshin', GameType.GIMI, '/mods');

      const syncCallArgs = vi
        .mocked(invoke)
        .mock.calls.find((c) => c[0] === 'deepmatch_scanner_cmd')?.[1] as Record<string, unknown>;
      expect(syncCallArgs.onProgress).toBeInstanceOf(Channel);
    });
  });

  describe('runDeepmatchPreview', () => {
    it('should fetch master db and invoke deepmatch_preview_cmd', async () => {
      vi.mocked(invoke).mockImplementation(async (cmd) => {
        if (cmd === 'get_master_db') return '[]';
        return [];
      });

      const onEvent = vi.fn();
      await scanService.runDeepmatchPreview('g1', GameType.GIMI, '/mods', onEvent, ['/specific']);

      const previewCallArgs = vi
        .mocked(invoke)
        .mock.calls.find((c) => c[0] === 'deepmatch_preview_cmd')?.[1] as Record<string, unknown>;
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

  describe('runDeepmatchPreviewForObjects', () => {
    it('should fetch master db and invoke deepmatch_preview_for_objects_cmd', async () => {
      vi.mocked(invoke).mockImplementation(async (cmd) => {
        if (cmd === 'get_master_db') return '[]';
        return [];
      });

      const onEvent = vi.fn();
      await scanService.runDeepmatchPreviewForObjects(
        'g1',
        GameType.GIMI,
        '/mods',
        ['obj1', 'obj2'],
        onEvent,
      );

      const previewCallArgs = vi
        .mocked(invoke)
        .mock.calls.find((c) => c[0] === 'deepmatch_preview_for_objects_cmd')?.[1] as Record<
        string,
        unknown
      >;
      expect(previewCallArgs.input).toEqual({
        gameId: 'g1',
        modsPath: '/mods',
        dbJson: '[]',
        objectIds: ['obj1', 'obj2'],
      });
      expect(previewCallArgs.onProgress).toBeInstanceOf(Channel);

      const channel = previewCallArgs.onProgress as import('@tauri-apps/api/core').Channel<{
        type: string;
      }>;
      channel.onmessage({ type: 'progress' });
      expect(onEvent).toHaveBeenCalledWith({ type: 'progress' });
    });
  });

  describe('commitScan', () => {
    it('should invoke commit_scan_cmd', async () => {
      const items = [
        { folderPath: '/a', skip: false } as unknown as Parameters<
          typeof scanService.commitScan
        >[4][0],
      ];
      await scanService.commitScan('g1', 'Genshin', GameType.GIMI, '/mods', items);
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

      const result = await scanService.scoreCandidatesBatch(
        '/mods/MyMod',
        ['Candidate1'],
        GameType.GIMI,
      );

      expect(invoke).toHaveBeenCalledWith('score_candidates_batch_cmd', {
        folderPath: '/mods/MyMod',
        candidateNames: ['Candidate1'],
        dbJson: '[]',
      });
      expect(result).toEqual({ Candidate1: 85 });
    });
  });
});
