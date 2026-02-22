import { invoke, Channel } from '@tauri-apps/api/core';
import type {
  ArchiveInfo,
  ArchiveAnalysis,
  ExtractionResult,
  ScanResultItem,
  ScanEvent,
  ConflictInfo,
} from '../types/scanner';

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
   * Extract an archive.
   */
  async extractArchive(
    archivePath: string,
    modsDir: string,
    password?: string,
    overwrite: boolean = false,
  ): Promise<ExtractionResult> {
    return invoke('extract_archive_cmd', {
      archivePath,
      modsDir,
      password: password || null,
      overwrite,
    });
  },

  /**
   * Analyze an archive before extraction.
   */
  async analyzeArchive(archivePath: string): Promise<ArchiveAnalysis> {
    return invoke('analyze_archive_cmd', { archivePath });
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
};

export interface SyncResult {
  total_scanned: number;
  new_mods: number;
  updated_mods: number;
  deleted_mods: number;
  new_objects: number;
}
