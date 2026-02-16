/**
 * Types for Epic 2: Mod Scanner & Organization.
 * Matches Rust structs in `src-tauri/src/commands/scan_cmds.rs`.
 */

export interface ArchiveInfo {
  path: string;
  name: string;
  extension: string;
  size_bytes: number;
  has_ini: boolean | null;
}

export interface ArchiveAnalysis {
  path: string;
  file_count: number;
  top_level_folders: string[];
  has_ini: boolean;
  total_size_bytes: number;
}

export interface ExtractionResult {
  success: boolean;
  extracted_path: string;
  files_extracted: number;
  error?: string;
}

export interface ScanResultItem {
  path: string;
  rawName: string;
  displayName: string;
  isDisabled: boolean;
  matchedObject: string | null;
  matchLevel: 'L1-Name' | 'L2-Token' | 'L3-Content' | 'L4-AI' | 'L5-Fuzzy' | 'Unmatched';
  confidence: 'High' | 'Medium' | 'Low' | 'None';
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

export type ScanEvent =
  | { event: 'started'; data: { totalFolders: number } }
  | { event: 'progress'; data: { current: number; folderName: string } }
  | { event: 'matched'; data: { folderName: string; objectName: string; confidence: string } }
  | { event: 'finished'; data: { matched: number; unmatched: number } };
