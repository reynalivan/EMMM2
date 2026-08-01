export type {
  ArchiveAnalysis,
  ArchiveEntryInfo,
  BulkActionError,
  BulkResult,
  CollisionInfo,
  ConfirmedScanItem,
  ConflictDetails,
  ConflictInfo,
  DeleteModResult,
  DupScanEvent,
  DupScanGroup,
  DupScanMember,
  DupScanReport,
  DupScanSignal,
  ExtractionEvent,
  ExtractionResult,
  FileEntry,
  FolderDetail,
  FolderEntry,
  IgnoredConflict,
  MatchCheckResult,
  MetadataSyncResult,
  ResolutionAction,
  ResolutionError,
  ResolutionRequest,
  ResolutionSummary,
  ScanEvent,
  ScoredCandidate,
  SyncResult,
  TrashMetadata,
  WhitelistEntry,
} from '../lib/bindings.gen';

import type {
  ArchiveEntryInfo as GenArchiveEntryInfo,
  ArchiveInfo as GenArchiveInfo,
  ScanPreviewItem as GenScanPreviewItem,
} from '../lib/bindings.gen';

/** FE enrichment: analysis entries get attached to the archive row after analyze. */
export type ArchiveInfo = GenArchiveInfo & { entries?: GenArchiveEntryInfo[] };

/** FE enrichment: flag set by the temp-import flow, not part of the wire payload. */
export type ScanPreviewItem = GenScanPreviewItem & { moveFromTemp?: boolean };

export type DuplicateInfo = {
  mod_id: string;
  object_id: string;
  folder_path: string;
  actual_name: string;
  is_variant: boolean;
  parent_path: string;
};

/**
 * UI-only vocabulary for the duplicate report: the user picks per GROUP, while
 * the Rust wire contract is per PAIR. `buildResolutionRequests` translates.
 */
export type DuplicateSelection = { type: 'Keep'; targetPath: string } | { type: 'Ignore' } | null;

/** MasterDB entry used by the scan-review override search and mod-runtime import. */
export interface MasterDbEntry {
  matched_entry_key: string;
  name: string;
  object_type: string;
  tags: string[];
  metadata: Record<string, unknown> | null;
  thumbnail_path: string | null;
}
