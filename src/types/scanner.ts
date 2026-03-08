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

export interface ConflictInfo {
  hash: string;
  section_name: string;
  mod_paths: string[];
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
