import { Channel } from '@tauri-apps/api/core';
import { commands } from '../bindings';
import type {
  ArchiveInfo,
  ArchiveAnalysis,
  ExtractionResult,
  ExtractionEvent,
  ScanEvent,
  ConflictInfo,
  SyncResult,
  ScanPreviewItem,
  MatchCheckResult,
  ConfirmedScanItem,
} from '../../types/scanner';
import type { GameType } from '../../types/game';
import { getGameTypeKey } from '../../types/game';

export type { ScanPreviewItem, ConfirmedScanItem };

export const scanService = {
  /**
   * Get the MasterDB JSON for a game type (e.g. "GIMI", "SRMI").
   */
  async getMasterDb(gameType: GameType): Promise<string> {
    return commands.getMasterDb(gameType);
  },

  /**
   * Detect archives in the mod directory.
   */
  async detectArchives(modsPath: string): Promise<ArchiveInfo[]> {
    return commands.detectArchivesCmd(modsPath);
  },

  /**
   * Extract an archive. Optionally stream per-file progress via Channel.
   */
  async extractArchive(
    archivePath: string,
    modsDir: string,
    password?: string,
    overwrite: boolean = false,
    customName?: string,
    disableAfter: boolean = false,
    unpackNested: boolean = true,
    onFileProgress?: (event: ExtractionEvent) => void,
  ): Promise<ExtractionResult> {
    const channel = new Channel<ExtractionEvent>();
    if (onFileProgress) {
      channel.onmessage = onFileProgress;
    }
    return commands.extractArchiveCmd(
      archivePath,
      modsDir,
      password || null,
      overwrite,
      customName || null,
      disableAfter,
      unpackNested,
      channel,
    );
  },

  /**
   * Extract multiple archives sequentially.
   */
  async extractArchiveBatch(
    selectedPaths: string[],
    archives: ArchiveInfo[],
    modsDir: string,
    passwords: Record<string, string>,
    options?: {
      autoRename?: boolean;
      disableByDefault?: boolean;
      folderNames?: Record<string, string>;
      unpackNested?: boolean;
    },
    onProgress?: (current: number, total: number) => void,
    onFileProgress?: (event: ExtractionEvent) => void,
  ): Promise<{
    extractedPaths: string[];
    aborted: boolean;
    results: Array<{
      path: string;
      status: 'done' | 'failed' | 'skipped' | 'aborted';
      error?: string;
      destPaths?: string[];
    }>;
    failedPath?: string;
    isPasswordError?: boolean;
    error?: string;
  }> {
    const overwrite = options?.autoRename === false;
    const disableAfter = options?.disableByDefault ?? false;
    const unpackNested = options?.unpackNested ?? true;
    const folderNames = options?.folderNames ?? {};
    const extractedPaths: string[] = [];
    const results: Array<{
      path: string;
      status: 'done' | 'failed' | 'skipped' | 'aborted';
      error?: string;
      destPaths?: string[];
    }> = [];

    const nonEncrypted = selectedPaths.filter(
      (p) => !archives.find((a) => a.path === p)?.is_encrypted,
    );
    const encrypted = selectedPaths.filter(
      (p) => !!archives.find((a) => a.path === p)?.is_encrypted,
    );

    const ordered = [...nonEncrypted, ...encrypted];
    const total = ordered.length;
    let current = 0;

    const PASSWORD_HINTS = ['password', 'encrypt', 'decrypt', 'wrong key', 'invalid key'];
    const isPasswordRelated = (err: string) =>
      PASSWORD_HINTS.some((h) => err.toLowerCase().includes(h));

    for (const archivePath of ordered) {
      const isEnc = !!archives.find((a) => a.path === archivePath)?.is_encrypted;
      const pw = isEnc ? passwords[archivePath] : undefined;
      const customName = folderNames[archivePath] || undefined;

      try {
        const result = await this.extractArchive(
          archivePath,
          modsDir,
          pw,
          overwrite,
          customName,
          disableAfter,
          unpackNested,
          onFileProgress,
        );

        if (result.aborted) {
          results.push({ path: archivePath, status: 'aborted' });
          for (const remaining of ordered.slice(current + 1)) {
            results.push({ path: remaining, status: 'skipped' });
          }
          return { extractedPaths, aborted: true, results };
        }

        if (!result.success) {
          const errMsg = result.error ?? 'Unknown error';
          if (isPasswordRelated(errMsg)) {
            results.push({ path: archivePath, status: 'failed', error: errMsg });
            for (const remaining of ordered.slice(current + 1)) {
              results.push({ path: remaining, status: 'skipped' });
            }
            return {
              extractedPaths,
              aborted: false,
              results,
              failedPath: archivePath,
              isPasswordError: true,
              error: errMsg,
            };
          }
          results.push({ path: archivePath, status: 'failed', error: errMsg });
        } else {
          const destPaths = result.destPaths ?? [];
          extractedPaths.push(...destPaths);
          results.push({ path: archivePath, status: 'done', destPaths });
        }
      } catch (e) {
        const errMsg = e instanceof Error ? e.message : String(e);
        if (isPasswordRelated(errMsg)) {
          results.push({ path: archivePath, status: 'failed', error: errMsg });
          for (const remaining of ordered.slice(current + 1)) {
            results.push({ path: remaining, status: 'skipped' });
          }
          return {
            extractedPaths,
            aborted: false,
            results,
            failedPath: archivePath,
            isPasswordError: true,
            error: errMsg,
          };
        }
        results.push({ path: archivePath, status: 'failed', error: errMsg });
      }

      current++;
      onProgress?.(current, total);
    }

    return { extractedPaths, aborted: false, results };
  },

  /**
   * Analyze an archive before extraction.
   */
  async analyzeArchive(archivePath: string): Promise<ArchiveAnalysis> {
    return commands.analyzeArchiveCmd(archivePath);
  },

  /**
   * Run a lightweight match check against an extracted folder.
   */
  async matchCheckFolder(
    folderPath: string,
    targetObjectName: string,
    gameType: GameType,
  ): Promise<MatchCheckResult> {
    return commands.matchCheckFolderCmd(folderPath, targetObjectName, gameType);
  },

  /**
   * Detect conflicts in INI files.
   */
  async detectConflicts(iniPaths: string[]): Promise<ConflictInfo[]> {
    return commands.detectConflictsCmd(iniPaths);
  },

  /**
   * Detect conflicts in the entire mods folder.
   */
  async detectConflictsInFolder(modsPath: string): Promise<ConflictInfo[]> {
    return commands.detectConflictsInFolderCmd(modsPath);
  },

  /**
   * Cancel the currently running scan.
   */
  async cancelScan(): Promise<void> {
    return commands.cancelScanCmd();
  },

  /**
   * Deep Match Scanner (scan + canonical import in one step).
   */
  async runDeepmatchScanner(
    gameId: string,
    gameName: string,
    gameType: GameType,
    modsPath: string,
    onEvent?: (event: ScanEvent) => void,
  ): Promise<SyncResult> {
    const channel = new Channel<ScanEvent>();
    if (onEvent) {
      channel.onmessage = onEvent;
    }
    return commands.deepmatchScannerCmd(
      gameId,
      gameName,
      getGameTypeKey(gameType),
      gameType,
      modsPath,
      false,
      channel,
    );
  },

  /**
   * Deep Match Scanner preview.
   */
  async runDeepmatchPreview(
    gameId: string,
    gameType: GameType,
    modsPath: string,
    onEvent?: (event: ScanEvent) => void,
    specificPaths?: string[],
  ): Promise<ScanPreviewItem[]> {
    const channel = new Channel<ScanEvent>();
    if (onEvent) {
      channel.onmessage = onEvent;
    }
    return commands.deepmatchPreviewCmd(
      gameId,
      modsPath,
      gameType,
      channel,
      specificPaths ?? null,
    );
  },

  /**
   * Deep Match Scanner preview for selected workspace objects.
   */
  async runDeepmatchPreviewForObjects(
    gameId: string,
    gameType: GameType,
    modsPath: string,
    objectIds: string[],
    onEvent?: (event: ScanEvent) => void,
  ): Promise<ScanPreviewItem[]> {
    const channel = new Channel<ScanEvent>();
    if (onEvent) {
      channel.onmessage = onEvent;
    }
    return commands.deepmatchPreviewForObjectsCmd(
      {
        gameId,
        modsPath,
        gameType,
        objectIds,
      },
      channel,
    );
  },

  /**
   * Phase 2: Commit user-confirmed scan results to DB.
   */
  async commitScan(
    gameId: string,
    gameName: string,
    gameType: GameType,
    modsPath: string,
    items: ConfirmedScanItem[],
  ): Promise<SyncResult> {
    return commands.commitScanCmd(gameId, gameName, getGameTypeKey(gameType), modsPath, items);
  },

  /**
   * Score a folder against candidate object names.
   */
  async scoreCandidatesBatch(
    folderPath: string,
    candidateNames: string[],
    gameType: GameType,
  ): Promise<Partial<Record<string, number>>> {
    return commands.scoreCandidatesBatchCmd(folderPath, candidateNames, gameType);
  },
};
