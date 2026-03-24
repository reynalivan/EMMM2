/**
 * Types for Epic 2: Mod Scanner & Organization.
 * Matches Rust structs in `src-tauri/src/commands/scan_cmds.rs`.
 */

export interface ArchiveInfo {
  path: string;
  name: string;
  extension: string;
  size_bytes: number;
  has_ini: boolean;
  file_count: number;
  /** Whether the archive requires a password for extraction. */
  is_encrypted: boolean;
  contains_nested_archives: boolean;
  /** File entries for tree preview (from analysis, capped at 500). */
  entries?: Array<{ path: string; isDir: boolean; size: number }>;
}

export interface ArchiveAnalysis {
  format: string;
  file_count: number;
  has_ini: boolean;
  uncompressed_size: number;
  file_size_bytes: number;
  single_root_folder: string | null;
  is_encrypted: boolean;
  contains_nested_archives: boolean;
  entries: Array<{ path: string; isDir: boolean; size: number }>;
}

export type CollisionResolution = 'rename' | 'skip' | 'overwrite' | 'merge';

export interface CollisionInfo {
  id: string;
  sourcePath: string;
  targetPath: string;
  objectName: string;
  existingModId: string | null;
}

export interface ExtractionResult {
  archive_name: string;
  /** Primary destination path (backward compat — first mod root moved). */
  dest_path: string;
  /** All destination paths (for multi-mod packs, may be > 1). */
  dest_paths: string[];
  files_extracted: number;
  /** Number of independent mod roots found and moved. */
  mod_count: number;
  success: boolean;
  error?: string;
  aborted?: boolean;
  collisions: CollisionInfo[];
}

export interface ScanResultItem {
  path: string;
  rawName: string;
  displayName: string;
  isDisabled: boolean;
  matchedObject: string | null;
  matchLevel: 'AutoMatched' | 'NeedsReview' | 'NoMatch';
  confidence: 'High' | 'Medium' | 'Low' | 'None';
  confidenceScore: number;
  matchDetail: string | null;
  detectedSkin: string | null;
  /** Canonical folder name for this skin variant (first alias). */
  skinFolderName: string | null;
  thumbnailPath: string | null;
}

export type ScoredCandidate = {
  name: string;
  objectType: string;
  scorePct: number;
};

export type ScanPreviewItem = {
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
  scoredCandidates: ScoredCandidate[];
  hashDbJson: string | null;
  customSkinsJson: string | null;
  dbThumbnail: string | null;
};

export interface SyncResult {
  total_scanned: number;
  new_mods: number;
  updated_mods: number;
  deleted_mods: number;
  new_objects: number;
  collisions: CollisionInfo[];
}

export interface MetadataSyncResult {
  success: boolean;
  updated: boolean;
  version?: string;
  updated_count: number;
  failed_count: number;
  errors?: string[];
}

export interface FileEntry {
  name: string;
  size: number;
  is_ini: boolean;
}

export interface FolderDetail {
  path: string;
  folder_name: string;
  is_enabled: boolean;
  total_size: number;
  file_count: number;
  files: FileEntry[];
  thumbnail_path: string | null;
}

export interface ConflictDetails {
  enabled: FolderDetail;
  disabled: FolderDetail;
}

export interface ConflictInfo {
  hash: string;
  section_name: string;
  mod_paths: string[];
  is_active: boolean;
}

export interface MatchCheckResult {
  matchedName: string | null;
  matchScorePct: number;
  targetScorePct: number;
  isMatch: boolean;
  confidence: string;
}

export type ScanEvent =
  | { event: 'started'; data: { totalFolders: number } }
  | { event: 'progress'; data: { current: number; folderName: string; etaMs: number } }
  | { event: 'matched'; data: { folderName: string; objectName: string; confidence: string } }
  | { event: 'finished'; data: { matched: number; unmatched: number } };

export type ExtractionEvent = {
  event: 'fileProgress';
  data: { fileName: string; fileIndex: number; totalFiles: number };
};

// Extracted from bindings.ts
export type TrashMetadata = {
  id: string;
  original_path: string;
  original_name: string;
  deleted_at: string;
  size_bytes: number;
  game_id: string | null;
};

export type BulkActionError = {
  path: string;
  error: string;
};

export type BulkResult = {
  success: string[];
  failures: BulkActionError[];
};

export type DuplicateInfo = {
  mod_id: string;
  object_id: string;
  folder_path: string;
  actual_name: string;
  is_variant: boolean;
  parent_path: string;
};

export type IgnoredConflict = {
  id: string;
  game_id: string;
  object_id: string;
  object_name?: string;
  mod_ids: string[];
  mod_names: string[];
};

export type DupScanSignal = {
  key: string;
  detail: string;
  score: number;
};

export type DupScanMember = {
  modId: string | null;
  folderPath: string;
  displayName: string;
  totalSizeBytes: number;
  fileCount: number;
  isSafe: boolean;
  confidenceScore: number;
  signals: DupScanSignal[];
};

export type DupScanGroup = {
  groupId: string;
  confidenceScore: number;
  matchReason: string;
  isUnsafe: boolean;
  signals: DupScanSignal[];
  members: DupScanMember[];
};

export type DupScanReport = {
  scanId: string;
  gameId: string;
  rootPath: string;
  totalGroups: number;
  totalMembers: number;
  groups: DupScanGroup[];
};

export type DupScanEvent =
  | { event: 'started'; data: { scanId: string; gameId: string; totalFolders: number } }
  | {
      event: 'progress';
      data: {
        scanId: string;
        processedFolders: number;
        totalFolders: number;
        currentFolder: string;
        percent: number;
      };
    }
  | { event: 'match'; data: { scanId: string; group: DupScanGroup } }
  | { event: 'finished'; data: { scanId: string; totalGroups: number; totalMembers: number } }
  | {
      event: 'cancelled';
      data: { scanId: string; processedFolders: number; totalFolders: number };
    };

export type ResolutionAction = { type: 'Keep'; targetPath: string } | { type: 'Ignore' } | null;

export type ResolutionError = {
  groupId: string;
  action: ResolutionAction;
  message: string;
};

export type ResolutionSummary = {
  total: number;
  successful: number;
  failed: number;
  errors: ResolutionError[];
};

export type ResolutionRequest = {
  groupId: string;
  action: 'Keep' | 'Ignore';
  targetPath?: string; // Only if action is 'Keep'
  allMembers: string[]; // All folder paths in the group
};

export type ConfirmedScanItem = {
  folderPath: string;
  displayName: string;
  isDisabled: boolean;
  matchedObject: string | null;
  objectType: string | null;
  thumbnailPath: string | null;
  tagsJson: string | null;
  metadataJson: string | null;
  hashDbJson: string | null;
  customSkinsJson: string | null;
  dbThumbnail: string | null;
  skip: boolean;
  moveFromTemp?: boolean;
};

export type MetadataDraftValues = {
  object_type: string;
  sub_category?: string;
  metadata?: Record<string, string>;
};

export type FolderEntry = {
  name: string;
  is_dir: boolean;
};

export type WhitelistEntry = {
  id: string;
  folder_a_id: string;
  folder_b_id: string;
  folder_a_name: string;
  folder_b_name: string;
  reason: string;
  ignored_at: string;
};
