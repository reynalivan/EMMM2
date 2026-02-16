/**
 * Types for Epic 9: Duplicate Scanner.
 * TypeScript mirrors of Rust contracts in src-tauri/src/types/dup_scan.rs.
 * All field names transformed from snake_case to camelCase per serde(rename_all).
 */

/**
 * Streaming event variants for duplicate scan progress.
 * Mirrors: DupScanEvent (Rust enum with tag="event", content="data")
 */
export type DupScanEvent =
  | {
      event: 'Started';
      data: {
        scanId: string;
        gameId: string;
        totalFolders: number;
      };
    }
  | {
      event: 'Progress';
      data: {
        scanId: string;
        processedFolders: number;
        totalFolders: number;
        currentFolder: string;
        percent: number;
      };
    }
  | {
      event: 'Match';
      data: {
        scanId: string;
        group: DupScanGroup;
      };
    }
  | {
      event: 'Finished';
      data: {
        scanId: string;
        totalGroups: number;
        totalMembers: number;
      };
    }
  | {
      event: 'Cancelled';
      data: {
        scanId: string;
        processedFolders: number;
        totalFolders: number;
      };
    };

/**
 * Root scan report structure.
 * Contains aggregated metrics and all duplicate groups found.
 */
export interface DupScanReport {
  scanId: string;
  gameId: string;
  rootPath: string;
  totalGroups: number;
  totalMembers: number;
  groups: DupScanGroup[];
}

/**
 * A cluster of potential duplicate mods (2..N members).
 * Group-based design: scores and signals aggregate evidence.
 */
export interface DupScanGroup {
  groupId: string;
  confidenceScore: number;
  matchReason: string;
  signals: DupScanSignal[];
  members: DupScanMember[];
}

/**
 * One mod folder entry inside a duplicate group.
 * Includes folder path, metadata, and individual signals.
 */
export interface DupScanMember {
  modId?: string;
  folderPath: string;
  displayName: string;
  totalSizeBytes: number;
  fileCount: number;
  confidenceScore: number;
  signals: DupScanSignal[];
}

/**
 * Evidence signal for duplicate detection.
 * Used in both group-level and member-level scopes.
 */
export interface DupScanSignal {
  key: string;
  detail: string;
  score: number;
}

/**
 * User action for resolving a duplicate pair.
 * KeepA: delete B | KeepB: delete A | Ignore: whitelist pair
 */
export type ResolutionAction = 'KeepA' | 'KeepB' | 'Ignore';

/**
 * Request to resolve one duplicate group.
 * Specifies which folder to keep and which to delete/ignore.
 */
export interface ResolutionRequest {
  groupId: string;
  action: ResolutionAction;
  folderA: string;
  folderB: string;
}

/**
 * Error detail for a failed resolution.
 */
export interface ResolutionError {
  groupId: string;
  message: string;
}

/**
 * Summary of batch resolution results.
 */
export interface ResolutionSummary {
  total: number;
  successful: number;
  failed: number;
  errors: ResolutionError[];
}

/**
 * Progress event emitted during bulk resolution.
 * Used for UI progress bars during multi-group resolution.
 */
export interface ResolutionProgress {
  current: number;
  total: number;
  groupId: string;
  action: string;
}
