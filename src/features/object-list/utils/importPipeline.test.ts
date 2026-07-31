import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { GameConfig } from '../../../types/game';
import { GameType } from '../../../types/game';
import {
  buildScanReviewState,
  ensureDirectoryExists,
  ingestLooseFiles,
  tempStagingPath,
  warnOnFolderMismatch,
} from './importPipeline';

const checkPathExists = vi.fn();
const ensureDir = vi.fn();
const ingestDroppedFolders = vi.fn();
const runDeepmatchPreview = vi.fn();
const getMasterDb = vi.fn();
const matchCheckFolder = vi.fn();
const toastWithAction = vi.fn();

vi.mock('../../../lib/bindings', () => ({
  sparse: (value: unknown) => value,
  commands: {
    checkPathExistsCmd: (...args: unknown[]) => checkPathExists(...args),
    ensureDirCmd: (...args: unknown[]) => ensureDir(...args),
    ingestDroppedFolders: (...args: unknown[]) => ingestDroppedFolders(...args),
  },
}));

vi.mock('../../../lib/services/scanService', () => ({
  scanService: {
    runDeepmatchPreview: (...args: unknown[]) => runDeepmatchPreview(...args),
    getMasterDb: (...args: unknown[]) => getMasterDb(...args),
    matchCheckFolder: (...args: unknown[]) => matchCheckFolder(...args),
  },
}));

vi.mock('../../mod-runtime/operations/sharedOperations', () => ({
  parseMasterDb: (json: string) => JSON.parse(json),
}));

vi.mock('../../../stores/useToastStore', () => ({
  toast: {
    withAction: (...args: unknown[]) => toastWithAction(...args),
  },
}));

const activeGame = {
  id: 'game-1',
  name: 'Genshin',
  game_type: GameType.GIMI,
  mod_path: 'E:\\Mods',
} as unknown as GameConfig;

describe('importPipeline', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    checkPathExists.mockResolvedValue(false);
    ensureDir.mockResolvedValue(undefined);
    getMasterDb.mockResolvedValue('[]');
  });

  it('derives the staging path from the mods root', () => {
    expect(tempStagingPath('E:\\Mods')).toBe('E:\\Mods\\.emmm_temp');
  });

  describe('ensureDirectoryExists', () => {
    it('creates the directory only when it is missing', async () => {
      await ensureDirectoryExists('E:\\Mods\\.emmm_temp');
      expect(ensureDir).toHaveBeenCalledWith('E:\\Mods\\.emmm_temp');

      checkPathExists.mockResolvedValue(true);
      ensureDir.mockClear();
      await ensureDirectoryExists('E:\\Mods\\.emmm_temp');
      expect(ensureDir).not.toHaveBeenCalled();
    });
  });

  describe('ingestLooseFiles', () => {
    it('skips every filesystem call when nothing is loose', async () => {
      await expect(ingestLooseFiles([], 'E:\\Mods\\.emmm_temp')).resolves.toEqual([]);
      expect(checkPathExists).not.toHaveBeenCalled();
      expect(ingestDroppedFolders).not.toHaveBeenCalled();
    });

    it('stages loose files and returns the moved folders', async () => {
      ingestDroppedFolders.mockResolvedValue(['E:\\Mods\\.emmm_temp\\mod']);

      await expect(
        ingestLooseFiles(['E:\\Drop\\mod.ini'], 'E:\\Mods\\.emmm_temp'),
      ).resolves.toEqual(['E:\\Mods\\.emmm_temp\\mod']);

      expect(ensureDir).toHaveBeenCalledWith('E:\\Mods\\.emmm_temp');
      expect(ingestDroppedFolders).toHaveBeenCalledWith(
        ['E:\\Drop\\mod.ini'],
        'E:\\Mods\\.emmm_temp',
      );
    });
  });

  describe('buildScanReviewState', () => {
    it('flags staged folders as moveFromTemp and opens the review', async () => {
      runDeepmatchPreview.mockResolvedValue([
        { folderPath: 'E:\\Mods\\.emmm_temp\\mod' },
        { folderPath: 'E:\\Mods\\Existing' },
      ]);
      getMasterDb.mockResolvedValue('[{"key":"ayaka"}]');

      const state = await buildScanReviewState(activeGame, ['E:\\Mods\\.emmm_temp\\mod']);

      expect(state.open).toBe(true);
      expect(state.isCommitting).toBe(false);
      expect(state.items.map((item) => item.moveFromTemp)).toEqual([true, false]);
      expect(state.masterDbEntries).toEqual([{ key: 'ayaka' }]);
    });
  });

  describe('warnOnFolderMismatch', () => {
    const input = {
      activeGame,
      folders: ['E:\\Mods\\Yelan'],
      objectName: 'Ayaka',
      noun: 'archive(s)',
      logLabel: 'check failed:',
      onFix: vi.fn(),
    };

    it('stays silent when every folder matched', async () => {
      matchCheckFolder.mockResolvedValue({
        isMatch: true,
        matchedName: 'Ayaka',
        matchScorePct: 98,
      });

      await warnOnFolderMismatch(input);

      expect(toastWithAction).not.toHaveBeenCalled();
    });

    it('raises an actionable warning on mismatch', async () => {
      matchCheckFolder.mockResolvedValue({
        isMatch: false,
        matchedName: 'Yelan',
        matchScorePct: 41,
      });

      await warnOnFolderMismatch(input);

      expect(toastWithAction).toHaveBeenCalledTimes(1);
      expect(toastWithAction.mock.calls[0][1]).toBe(
        '1 of 1 archive(s) may not match Ayaka\n→ Yelan: Best match is Yelan (41%)',
      );
    });

    it('never fails the import when the check itself throws', async () => {
      matchCheckFolder.mockRejectedValue(new Error('db unavailable'));
      const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});

      await expect(warnOnFolderMismatch(input)).resolves.toBeUndefined();
      expect(toastWithAction).not.toHaveBeenCalled();

      warn.mockRestore();
    });
  });
});
