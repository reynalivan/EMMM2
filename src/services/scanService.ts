import { invoke, Channel } from '@tauri-apps/api/core';
import type {
  ArchiveInfo,
  ArchiveAnalysis,
  ExtractionResult,
  ScanResultItem,
  ScanEvent,
  ConflictInfo,
} from '../types/scanner';

// Initialize the MasterDB (will be loaded from file later)
// For now, we'll pass an empty DB or a minimal one
const EMPTY_DB_JSON = JSON.stringify({
  characters: [],
  weapons: [],
  ui: [],
  other: [],
});

export const scanService = {
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
   * @param onEvent Callback for progress events
   */
  async startScan(
    modsPath: string,
    onEvent: (event: ScanEvent) => void,
  ): Promise<ScanResultItem[]> {
    const channel = new Channel<ScanEvent>();

    channel.onmessage = (message) => {
      onEvent(message);
    };

    return invoke('start_scan', {
      modsPath,
      dbJson: EMPTY_DB_JSON, // TODO: Load actual DB
      onProgress: channel,
    });
  },

  /**
   * Get scan results without streaming (batch mode).
   */
  async getScanResult(modsPath: string): Promise<ScanResultItem[]> {
    return invoke('get_scan_result', {
      modsPath,
      dbJson: EMPTY_DB_JSON,
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
};
