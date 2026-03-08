import { invoke, Channel } from '@tauri-apps/api/core';
import type {
  ArchiveInfo,
  ArchiveAnalysis,
  ExtractionResult,
  ExtractionEvent,
  ScanResultItem,
  ScanEvent,
  ConflictInfo,
} from '../../types/scanner';

/** Preview item returned by scan_preview_cmd (before user confirms). */
export interface ScanPreviewItem {
  folderPath: string;
  displayName: string;
  isDisabled: boolean;
  matchedObject: string | null;
  matchLevel: string;
  confidence: string;
  confidenceScore: number;
  matchDetail: string | null;
  detectedSkin: string | null;
  objectType: string | null;
  thumbnailPath: string | null;
  tagsJson: string | null;
  metadataJson: string | null;
  alreadyInDb: boolean;
  alreadyMatched: boolean;
  scoredCandidates: Array<{ name: string; objectType: string; scorePct: number }>;
}

/** User-confirmed item sent to commit_scan_cmd. */
export interface ConfirmedScanItem {
  folderPath: string;
  displayName: string;
  isDisabled: boolean;
  matchedObject: string | null;
  objectType: string | null;
  thumbnailPath: string | null;
  tagsJson: string | null;
  metadataJson: string | null;
  skip: boolean;
}

export const scanService = {
  /**
   * Get the MasterDB JSON for a game type (e.g. "GIMI", "SRMI").
   */
  async getMasterDb(gameType: string): Promise<string> {
    return invoke('get_master_db', { gameType });
  },

  /**
   * Detect archives in the mod directory.
   */
  async detectArchives(modsPath: string): Promise<ArchiveInfo[]> {
    return invoke('detect_archives_cmd', { modsPath });
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
    return invoke('extract_archive_cmd', {
      archivePath,
      modsDir,
      password: password || null,
      overwrite,
      customName: customName || null,
      disableAfter,
      unpackNested,
      onProgress: channel,
    });
  },

  /**
   * A1: Shared batch extraction utility with queue model.
   * Splits archives into non-encrypted and encrypted, extracts sequentially.
   * A non-password failure continues to the next archive (queue resilience).
   * Password errors and user abort stop immediately for retry / cancellation.
   *
   * @returns Object with `extractedPaths`, per-archive `results`, and summary.
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
    /** Set when a password error stops the queue (for retry flow). */
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
          // User cancelled — mark this as aborted, remaining as skipped
          results.push({ path: archivePath, status: 'aborted' });
          for (const remaining of ordered.slice(current + 1)) {
            results.push({ path: remaining, status: 'skipped' });
          }
          return { extractedPaths, aborted: true, results };
        }

        if (!result.success) {
          const errMsg = result.error ?? 'Unknown error';

          // Password errors stop the queue for retry (#5 flow)
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

          // Non-password error → log and continue to next archive
          results.push({ path: archivePath, status: 'failed', error: errMsg });
        } else {
          const destPaths = result.dest_paths ?? [];
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

        // Non-password exception → log and continue
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
    return invoke('analyze_archive_cmd', { archivePath });
  },

  /**
   * Run a lightweight match check against an extracted folder to see if it belongs to a target object.
   * @param folderPath The path to the extracted mod folder
   * @param targetObjectName The expected object name (e.g., 'Keqing')
   * @param gameType The active game type (e.g., 'GIMI') to load the correct MasterDB
   */
  async matchCheckFolder(
    folderPath: string,
    targetObjectName: string,
    gameType: string,
  ): Promise<import('../../types/scanner').MatchCheckResult> {
    const dbJson = await scanService.getMasterDb(gameType);
    return invoke('match_check_folder_cmd', {
      folderPath,
      targetObjectName,
      dbJson,
    });
  },

  /**
   * Start the full scan pipeline with progress streaming.
   * @param gameType Game type code (e.g. "GIMI", "SRMI")
   * @param onEvent Callback for progress events
   */
  async startScan(
    gameType: string,
    modsPath: string,
    onEvent: (event: ScanEvent) => void,
  ): Promise<ScanResultItem[]> {
    const channel = new Channel<ScanEvent>();

    channel.onmessage = (message) => {
      onEvent(message);
    };

    const dbJson = await scanService.getMasterDb(gameType);

    return invoke('start_scan', {
      modsPath,
      dbJson,
      onProgress: channel,
    });
  },

  /**
   * Get scan results without streaming (batch mode).
   */
  async getScanResult(gameType: string, modsPath: string): Promise<ScanResultItem[]> {
    const dbJson = await scanService.getMasterDb(gameType);
    return invoke('get_scan_result', {
      modsPath,
      dbJson,
    });
  },

  /**
   * Detect conflicts in INI files.
   */
  async detectConflicts(iniPaths: string[]): Promise<ConflictInfo[]> {
    return invoke('detect_conflicts_cmd', { iniPaths });
  },

  /**
   * Detect conflicts in the entire mods folder.
   * Efficient alternative to detectConflicts that runs scanning on backend.
   */
  async detectConflictsInFolder(modsPath: string): Promise<ConflictInfo[]> {
    return invoke('detect_conflicts_in_folder_cmd', { modsPath });
  },

  /**
   * Cancel the currently running scan.
   */
  async cancelScan(): Promise<void> {
    return invoke('cancel_scan_cmd');
  },

  /**
   * Legacy sync: scan + commit in one step (with game upsert).
   * @param onEvent Callback for progress events
   */
  async syncDatabase(
    gameId: string,
    gameName: string,
    gameType: string,
    modsPath: string,
    onEvent?: (event: ScanEvent) => void,
  ): Promise<SyncResult> {
    const channel = new Channel<ScanEvent>();

    if (onEvent) {
      channel.onmessage = (message) => {
        onEvent(message);
      };
    }

    const dbJson = await scanService.getMasterDb(gameType);

    return invoke('sync_database_cmd', {
      gameId,
      gameName,
      gameType,
      modsPath,
      dbJson,
      onProgress: channel,
    });
  },

  /**
   * Phase 1: Scan folders + match, return preview without writing to DB.
   * Used by the review flow.
   */
  async scanPreview(
    gameId: string,
    gameType: string,
    modsPath: string,
    onEvent?: (event: ScanEvent) => void,
    specificPaths?: string[],
  ): Promise<ScanPreviewItem[]> {
    const channel = new Channel<ScanEvent>();

    if (onEvent) {
      channel.onmessage = (message) => {
        onEvent(message);
      };
    }

    const dbJson = await scanService.getMasterDb(gameType);

    return invoke('scan_preview_cmd', {
      gameId,
      modsPath,
      dbJson,
      onProgress: channel,
      specificPaths,
    });
  },

  /**
   * Quick import: scan + commit with EMPTY MasterDB so nothing matches.
   * All folders are imported as "Other" instantly. No Deep Matcher.
   */
  async quickImport(
    gameId: string,
    gameName: string,
    gameType: string,
    modsPath: string,
  ): Promise<SyncResult> {
    const channel = new Channel<ScanEvent>();
    return invoke('sync_database_cmd', {
      gameId,
      gameName,
      gameType,
      modsPath,
      dbJson: '[]',
      onProgress: channel,
    });
  },

  /**
   * Phase 2: Commit user-confirmed scan results to DB.
   */
  async commitScan(
    gameId: string,
    gameName: string,
    gameType: string,
    modsPath: string,
    items: ConfirmedScanItem[],
  ): Promise<SyncResult> {
    return invoke('commit_scan_cmd', {
      gameId,
      gameName,
      gameType,
      modsPath,
      items,
    });
  },

  /**
   * Score a folder against candidate object names.
   * Returns a map of candidateName → score (0-100%).
   * Used for pre-drop validation to check if the target is a good match.
   */
  async scoreCandidatesBatch(
    folderPath: string,
    candidateNames: string[],
    gameType: string,
  ): Promise<Record<string, number>> {
    const dbJson = await scanService.getMasterDb(gameType);
    return invoke('score_candidates_batch_cmd', {
      folderPath,
      candidateNames,
      dbJson,
    });
  },
};

export interface SyncResult {
  total_scanned: number;
  new_mods: number;
  updated_mods: number;
  deleted_mods: number;
  new_objects: number;
}
