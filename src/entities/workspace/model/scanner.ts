export type {
  BulkActionError,
  BulkResult,
  ConflictInfo,
  DeleteModResult,
  DupScanEvent,
  DupScanGroup,
  DupScanMember,
  DupScanReport,
  DupScanSignal,
  FolderNameConflictCandidate,
  FolderNameConflictGroup,
  FolderEntry,
  IgnoredConflict,
  MetadataSyncResult,
  ResolutionAction,
  ResolutionError,
  ResolutionRequest,
  ResolutionSummary,
  WhitelistEntry,
} from '@/shared/api/tauri/bindings.gen';

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
