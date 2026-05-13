/**
 * Types & IPC bindings for EMMM.
 * Directly sourced from `src/types/*` to avoid giant monolithic files.
 * Completely Type-Safe.
 */

import { invoke, Channel } from '@tauri-apps/api/core';

// Internal types that are too small for separate files
export type TaskStatus = 'PENDING' | 'COMPLETED' | 'FAILED';
export type PipelineTask = {
  id: string;
  game_id: string;
  task_type: string;
  status: TaskStatus;
  target_id: string | null;
  created_at: string;
  updated_at: string;
};

export type IngestResult = {
  moved: string[];
  failed: string[];
  skipped: string[];
};

export interface IniVariable {
  name: string;
  value: string;
  line_idx: number;
}

export interface KeyBinding {
  section_name: string;
  key: string | null;
  back: string | null;
  key_line_idx: number | null;
  back_line_idx: number | null;
}

export interface IniDocument {
  file_path: string;
  raw_lines: string[];
  variables: IniVariable[];
  key_bindings: KeyBinding[];
  had_bom: boolean;
  newline_style: 'Lf' | 'CrLf';
  mode: 'Structured' | 'RawFallback';
}

export interface IniLineUpdate {
  line_idx: number;
  content: string;
}

/** Shape returned by Rust `match_object_with_db` command. */
export interface MatchedDbEntry {
  name: string;
  matched_entry_key?: string | null;
  matched_alias_name?: string | null;
  object_type: string;
  tags: string[];
  metadata: Record<string, unknown> | null;
  thumbnail_path: string | null;
  /** Pipeline level: "L1Name" | "L2Token" | "L5Fuzzy" */
  match_level: string;
  /** Confidence: "High" | "Medium" | "Low" */
  match_confidence: string;
  /** Human-readable match detail */
  match_detail: string;
}

export interface ApplyObjectMatchInput {
  game_id: string;
  object_id?: string | null;
  folder_path?: string | null;
  matched_entry_key?: string | null;
  matched_alias_name?: string | null;
  matched_confidence?: number | null;
  matched_reason?: string | null;
  matched_source?: string | null;
}

export type WatcherState = { status: string; path: string | null; game_id: string | null };
export type DiskReconcileReason =
  | 'StartupBoot'
  | 'OnboardingCompleted'
  | 'ModsViewEntered'
  | 'WindowRefocused'
  | 'WatcherBatch'
  | 'ManualRepair'
  | 'GameSwitched'
  | 'InternalMutation';

export type DiskReconcilePathKind = 'Object' | 'Mod';
export type DiskReconcileStatus = 'Applied' | 'SourceUnavailable';

export interface DiskReconcilePathUpdate {
  from: string;
  to: string;
  kind: DiskReconcilePathKind;
}

export interface DiskReconcileChangeCounts {
  added: number;
  removed: number;
  renamed: number;
  modified: number;
}

export interface DiskReconcileChangeSummary {
  object_changes: DiskReconcileChangeCounts;
  mod_changes: DiskReconcileChangeCounts;
  object_sample_names: string[];
  mod_sample_names: string[];
  has_user_visible_changes: boolean;
}

export interface DiskReconcileResult {
  game_id: string;
  reason: DiskReconcileReason;
  status: DiskReconcileStatus;
  error_message: string | null;
  changed_roots: string[];
  objects_changed: boolean;
  folders_changed: boolean;
  collections_changed: boolean;
  runtime_file_changed: boolean;
  overlay_refresh_triggered: boolean;
  thumbnail_roots: string[];
  cleared_selection_paths: string[];
  path_updates: DiskReconcilePathUpdate[];
  change_summary: DiskReconcileChangeSummary;
}

// Domain Types
import type { FolderEntry } from '../types/scanner';
import type { GameConfig } from '../types/game';
import type {
  DbEntry,
  GameSchema,
  ObjectSummary,
  CategoryCount,
  FolderGridResponse,
  ModInfo,
  RenameResult,
  FolderContentInfo,
  ObjectFilter,
  CreateObjectInput,
  UpdateObjectInput,
  ModInfoUpdate,
} from '../types/object';
import type {
  WorkspaceSwitchInput,
  WorkspaceSwitchResult,
  WorkspaceViewModel,
  WorkspaceViewModelInput,
} from '../types/workspace';
import type {
  CollectionSummary,
  CorridorSnapshot,
  CorridorSwitchPreview,
  CollectionPreview,
  ApplyProgressSnapshot,
  ApplyPreview,
  ApplyResult,
  PinStatus,
  SwitchResult,
} from '../types/collection';
import type {
  ArchiveInfo,
  ArchiveAnalysis,
  ExtractionResult,
  ConflictInfo,
  ScanPreviewItem,
  ConflictDetails,
  SyncResult,
  TrashMetadata,
  BulkResult,
  DupScanReport,
  ResolutionSummary,
  MetadataSyncResult,
  MatchCheckResult,
  ResolutionRequest,
  ScanEvent,
  ExtractionEvent,
  DupScanEvent,
  ConfirmedScanItem,
  WhitelistEntry,
  IgnoredConflict,
} from '../types/scanner';
import type { DashboardPayload } from '../types/dashboard';
import type { AppSettings, ActiveKeyBinding } from '../types/settings';

export interface ThemeMetadata {
  id: string;
  label: string;
}

export interface ThemeConfig {
  colors: Record<string, string>;
  glass: Record<string, string>;
}

export interface CustomTheme {
  id: string;
  label: string;
  config: ThemeConfig;
}
import type { BrowserDownloadItem, ImportJobItem } from '../features/browser/types';
import type { RecoveryAction } from '../types/task';

export interface RandomModProposal {
  object_id: string;
  object_name: string;
  mod_id: string;
  name: string;
  display_name: string;
  thumbnail_path?: string | null;
  folder_path: string;
}

export interface DeepmatchPreviewForObjectsInput {
  gameId: string;
  modsPath: string;
  dbJson: string;
  objectIds: string[];
}

// ----- COMMANDS REGISTRY -----
export const commands = {
  // App & System
  appStartupCheck: () => invoke<PipelineTask[]>('app_startup_check'),
  checkBootSecurity: (params: { isSafeMode: boolean }) =>
    invoke<boolean>('check_boot_security', { isSafeMode: params.isSafeMode }),
  checkConfigStatus: () => invoke<boolean>('check_config_status'),
  checkMetadataUpdate: () => invoke<MetadataSyncResult>('check_metadata_update'),
  closeSplashscreen: () => invoke<void>('close_splashscreen'),
  getLogs: (params: { count?: number; limit?: number; offset?: number }) =>
    invoke<string[]>('get_logs', params),
  getLogLines: (params: { count?: number; limit?: number; offset?: number }) =>
    invoke<string[]>('get_logs', params),
  openLogFolder: () => invoke<void>('open_log_folder'),
  resetDatabase: () => invoke<void>('reset_database'),
  fetchMissingAsset: (params: { assetName: string }) => invoke<void>('fetch_missing_asset', params),
  checkPathExists: (params: { path: string }) => invoke<boolean>('check_path_exists_cmd', params),
  ensureDir: (params: { path: string }) => invoke<void>('ensure_dir_cmd', params),
  getSettings: () => invoke<AppSettings>('get_settings'),
  saveSettings: (params: { settings: AppSettings }) => invoke<void>('save_settings', params),
  runMaintenance: (params: { gameId?: string; id?: string }) =>
    invoke<string>('run_maintenance', params),
  clearOldThumbnails: () => invoke<string>('clear_old_thumbnails'),
  updateHotkeyConfig: (params: { config?: Record<string, string> }) =>
    invoke<void>('update_hotkey_config', params),
  getDashboardStats: (params: { gameId?: string; id?: string; safeMode?: boolean }) =>
    invoke<DashboardPayload>('get_dashboard_stats', params),
  getActiveKeybindings: (params: { gameId?: string; id?: string }) =>
    invoke<ActiveKeyBinding[]>('get_active_keybindings', params),
  getWorkspaceViewModel: (params: { input: WorkspaceViewModelInput }) =>
    invoke<WorkspaceViewModel>('get_workspace_view_model', params),
  executeWorkspaceSwitch: (params: { input: WorkspaceSwitchInput }) =>
    invoke<WorkspaceSwitchResult>('execute_workspace_switch', params),

  // Game Management
  getGames: () => invoke<GameConfig[]>('get_games'),
  autoDetectGames: (params?: { rootPath?: string }) =>
    invoke<GameConfig[]>('auto_detect_games', params),
  addGameManual: (params: { name?: string; path: string; gameType: number | string }) =>
    invoke<GameConfig>('add_game_manual', params),
  saveOnboardingGames: (params: { games: GameConfig[] }) =>
    invoke<void>('save_onboarding_games', params),
  removeGame: (params: { id?: string; gameId?: string }) => invoke<void>('remove_game', params),
  launchGame: (params: { id?: string; gameId?: string }) => invoke<void>('launch_game', params),
  setActiveGame: (params: { gameId?: string | null; id?: string | null }) =>
    invoke<void>('set_active_game', params),
  setAutoCloseLauncher: (params: { enabled: boolean }) =>
    invoke<void>('set_auto_close_launcher', params),

  // Master DB & Objects
  getGameSchema: (params: { gameType: number }) => invoke<GameSchema>('get_game_schema', params),
  getMasterDb: (params: { gameType: number }) => invoke<string>('get_master_db', params),
  searchMasterDb: (params: { gameType: number; query: string; objectType?: string | null }) =>
    invoke<{ score: number; item: DbEntry }[]>('search_master_db', params),
  getObject: (params: { id?: string; gameId?: string }) =>
    invoke<ObjectSummary>('get_object', params),
  // Note: getGame doesn't exist explicitly in lib.rs registry as _cmd, likely using get_games or a plugin.
  // Reverting to get_game if it was intended to exist via specta.
  getGame: (params: { id?: string; gameId?: string }) => invoke<GameConfig>('get_game', params),
  getObjects: (params?: { filter?: ObjectFilter } | { gameId?: string; safeMode?: boolean }) =>
    invoke<{ objects: ObjectSummary[]; lost_objects: string[] }>('get_objects_cmd', params),
  getCategoryCounts: (params?: { gameId?: string; safeMode?: boolean }) =>
    invoke<CategoryCount[]>('get_category_counts_cmd', params),
  createObject: (params: { input: CreateObjectInput }) =>
    invoke<string>('create_object_cmd', params),
  updateObject: (params: { id?: string; updates: UpdateObjectInput }) =>
    invoke<void>('update_object_cmd', params),
  applyObjectMatch: (params: { input: ApplyObjectMatchInput }) =>
    invoke<void>('apply_object_match_cmd', params),
  deleteObject: (params: { id?: string }) => invoke<void>('delete_object_cmd', params),
  pinObject: (params: { id?: string; isPinned?: boolean; pin?: boolean }) =>
    invoke<void>('pin_object', params),
  matchObjectWithDb: (params: { gameType: number; objectName: string }) =>
    invoke<MatchedDbEntry | null>('match_object_with_db', params),

  // Mod Management (Core)
  listModFolders: (params: {
    gameId?: string;
    id?: string;
    modsPath: string;
    subPath?: string;
    objectId?: string | null;
  }) => invoke<FolderGridResponse>('list_mod_folders', params),
  renameModFolder: (params: {
    folderPath?: string;
    path?: string;
    newName: string;
    gameId?: string;
  }) => invoke<RenameResult>('rename_mod_folder', params),
  deleteMod: (params: { path?: string; folderPath?: string; gameId?: string }) =>
    invoke<void>('delete_mod', params),
  preDeleteCheck: (params: { path?: string }) =>
    invoke<FolderContentInfo>('pre_delete_check', params),
  openInExplorer: (params: { gameId: string; path: string }) =>
    invoke<void>('open_in_explorer', params),
  revealObjectInExplorer: (params: { gameId: string; objectId: string; objectName: string }) =>
    invoke<string>('reveal_object_in_explorer', params),

  // Mod Metadata & Tags
  setModCategory: (params: {
    path?: string;
    folderPath?: string;
    category: string;
    gameId?: string;
  }) => invoke<void>('set_mod_category', params),
  setObjectModsCategory: (params: { gameId: string; objectId: string; category: string }) =>
    invoke<number>('set_object_mods_category', params),
  toggleModSafe: (params: {
    gameId?: string;
    id?: string;
    path?: string;
    folderPath?: string;
    safe?: boolean;
  }) => invoke<void>('toggle_mod_safe', params),
  toggleFavorite: (params: {
    path?: string;
    folderPath?: string;
    favorite?: boolean;
    gameId?: string;
  }) => invoke<void>('toggle_favorite', params),
  getActiveModConflicts: (params: { gameId?: string }) =>
    invoke<ConflictInfo[]>('get_active_mod_conflicts', params),
  suggestRandomMods: (params: { gameId: string; isSafe: boolean }) =>
    invoke<RandomModProposal[]>('suggest_random_mods', params),
  moveModToObject: (params: {
    gameId?: string;
    folderPath?: string;
    objectId?: string;
    targetObjectId?: string;
    status?: string;
  }) => invoke<void>('move_mod_to_object', params),

  // Previews & Ini
  readModInfo: (params: { folderPath?: string; path?: string }) =>
    invoke<ModInfo>('read_mod_info', params),
  updateModInfo: (params: { folderPath?: string; path?: string; update: ModInfoUpdate }) =>
    invoke<ModInfo>('update_mod_info', params),
  listModIniFiles: (params: { folderPath?: string; path?: string }) =>
    invoke<string[]>('list_mod_ini_files', params),
  readModIni: (params: { folderPath?: string; path?: string; fileName?: string }) =>
    invoke<IniDocument>('read_mod_ini', params),
  writeModIni: (params: {
    folderPath?: string;
    path?: string;
    fileName?: string;
    lineUpdates?: IniLineUpdate[];
    content?: string;
  }) => invoke<void>('write_mod_ini', params),
  listModPreviewImages: (params: { folderPath?: string; path?: string }) =>
    invoke<string[]>('list_mod_preview_images', params),
  saveModPreviewImage: (params: {
    folderPath?: string;
    modPath?: string;
    objectName?: string;
    imageData?: number[];
    imagePath?: string;
  }) => invoke<string>('save_mod_preview_image', params),
  removeModPreviewImage: (params: { folderPath?: string; imagePath?: string }) =>
    invoke<void>('remove_mod_preview_image', params),
  clearModPreviewImages: (params: { folderPath?: string }) =>
    invoke<string[]>('clear_mod_preview_images', params),

  // Thumbnails
  getThumbnail: (params: { folderPath?: string; path?: string }) =>
    invoke<string | null>('get_thumbnail', params),
  getModThumbnail: (params: { gameId: string; folderPath: string }) =>
    invoke<string | null>('get_mod_thumbnail', params),
  updateModThumbnail: (params: { folderPath?: string; path?: string; sourcePath: string }) =>
    invoke<string>('update_mod_thumbnail', params),
  deleteModThumbnail: (params: { folderPath?: string; path?: string }) =>
    invoke<void>('delete_mod_thumbnail', params),
  pasteThumbnail: (params: { folderPath?: string; path?: string; imageData: number[] }) =>
    invoke<string>('paste_thumbnail', params),

  // Bulk Operations
  bulkToggleMods: (params: { gameId: string; paths: string[]; enable: boolean }) =>
    invoke<BulkResult>('bulk_toggle_mods', params),
  bulkDeleteMods: (params: { paths: string[]; gameId?: string }) =>
    invoke<BulkResult>('bulk_delete_mods', params),
  bulkUpdateInfo: (params: { gameId: string; paths: string[]; update: ModInfoUpdate }) =>
    invoke<BulkResult>('bulk_update_info', params),
  bulkToggleFavorite: (params: { gameId: string; folderPaths: string[]; favorite: boolean }) =>
    invoke<BulkResult>('bulk_toggle_favorite', params),
  bulkPinMods: (params: { gameId: string; folderPaths: string[]; pin: boolean }) =>
    invoke<BulkResult>('bulk_pin_mods', params),

  // Scanner (Archives)
  detectArchives: (params: { modsPath: string }) =>
    invoke<ArchiveInfo[]>('detect_archives_cmd', params),
  extractArchive: (params: {
    archivePath: string;
    modsDir: string;
    password?: string | null;
    overwrite: boolean;
    customName?: string | null;
    disableAfter: boolean;
    unpackNested: boolean;
    onProgress: Channel<ExtractionEvent>;
  }) => invoke<ExtractionResult>('extract_archive_cmd', params),
  analyzeArchive: (params: { archivePath: string }) =>
    invoke<ArchiveAnalysis>('analyze_archive_cmd', params),
  matchCheckFolder: (params: { folderPath: string; targetObjectName: string; dbJson: string }) =>
    invoke<MatchCheckResult>('match_check_folder_cmd', params),
  listFolderEntries: (params: { folderPath: string; gameId: string }) =>
    invoke<FolderEntry[]>('list_folder_entries_cmd', params),
  abortExtraction: () => invoke<void>('abort_extraction_cmd'),

  // Scanner (General)
  cancelScan: () => invoke<void>('cancel_scan_cmd'),
  runDeepmatchScanner: (params: {
    gameId?: string;
    gameName?: string;
    gameType?: string;
    modsPath?: string;
    dbJson?: string;
    preserveExistingMappings?: boolean;
    onProgress?: Channel<ScanEvent>;
  }) => invoke<SyncResult>('deepmatch_scanner_cmd', params),
  runDeepmatchPreview: (params: {
    gameId: string;
    modsPath: string;
    dbJson: string;
    onProgress: Channel<ScanEvent>;
    specificPaths?: string[];
  }) => invoke<ScanPreviewItem[]>('deepmatch_preview_cmd', params),
  runDeepmatchPreviewForObjects: (params: {
    input: DeepmatchPreviewForObjectsInput;
    onProgress: Channel<ScanEvent>;
  }) => invoke<ScanPreviewItem[]>('deepmatch_preview_for_objects_cmd', params),
  commitScan: (params: {
    gameId?: string;
    gameName?: string;
    gameType?: string;
    modsPath?: string;
    items?: ConfirmedScanItem[];
  }) => invoke<SyncResult>('commit_scan_cmd', params),
  scoreCandidatesBatch: (params: {
    folderPath: string;
    candidateNames: string[];
    dbJson: string;
  }) => invoke<Record<string, number>>('score_candidates_batch_cmd', params),
  reconcileDiskState: (params: {
    gameId: string;
    reason: DiskReconcileReason;
    changedPaths?: string[];
    forceFull?: boolean;
  }) => invoke<DiskReconcileResult>('reconcile_disk_state_cmd', params),
  importModsFromPaths: (params: {
    paths: string[];
    targetDir: string;
    strategy: string;
    dbJson?: string;
  }) => invoke<BulkResult>('import_mods_from_paths', params),
  ingestDroppedFolders: (params: {
    paths?: string[];
    modsPath?: string;
    gameId?: string;
    gameName?: string;
    gameType?: string;
    dbJson?: string;
  }) => invoke<IngestResult>('ingest_dropped_folders', params),
  // Conflicts & Duplicates
  checkShaderConflicts: (params: { folderPath: string }) =>
    invoke<ConflictInfo[]>('check_shader_conflicts', params),
  detectConflicts: (params: { iniPaths: string[] }) =>
    invoke<ConflictInfo[]>('detect_conflicts_cmd', params),
  detectConflictsInFolder: (params: { modsPath: string }) =>
    invoke<ConflictInfo[]>('detect_conflicts_in_folder_cmd', params),
  getConflictDetails: (params: { path?: string; enabledPath?: string; disabledPath?: string }) =>
    invoke<ConflictDetails>('get_conflict_details', params),
  resolveConflict: (params: {
    path?: string;
    resolution?: string;
    keepPath?: string;
    duplicatePath?: string;
    strategy?: string;
  }) => invoke<void>('resolve_conflict', params),
  ignoreObjectConflict: (params: { gameId: string; objectId: string; modIds: string[] }) =>
    invoke<void>('ignore_object_conflict', params),
  revokeObjectConflict: (params: { gameId: string; objectId: string }) =>
    invoke<void>('revoke_object_conflict', params),
  listIgnoredObjectConflicts: (params: { gameId: string }) =>
    invoke<IgnoredConflict[]>('list_ignored_object_conflicts', params),

  // Duplicates Scanner
  dupScanStart: (params: { gameId: string; modsRoot: string; onEvent: Channel<DupScanEvent> }) =>
    invoke<void>('dup_scan_start', params),
  dupScanCancel: () => invoke<void>('dup_scan_cancel'),
  dupScanGetReport: (params: { pin?: string }) =>
    invoke<DupScanReport | null>('dup_scan_get_report', params),
  dupResolveBatch: (params: { gameId: string; requests: ResolutionRequest[] }) =>
    invoke<ResolutionSummary>('dup_resolve_batch', params),
  getIgnoredPairs: (params: { gameId: string }) =>
    invoke<WhitelistEntry[]>('get_ignored_pairs', params),
  removeIgnoredPair: (params: { entryId: string }) => invoke<number>('remove_ignored_pair', params),

  // Watcher
  startWatcher: (params: { gameId: string; path: string }) => invoke<void>('start_watcher', params),
  stopWatcher: () => invoke<void>('stop_watcher'),
  setWatcherSuppression: (params: { suppressed: boolean }) =>
    invoke<void>('set_watcher_suppression', params),
  getWatcherState: () => invoke<WatcherState>('get_file_watcher_state'),

  // Collections
  getCorridorState: (params: { gameId: string; isSafe: boolean }) =>
    invoke<CorridorSnapshot>('get_corridor_state', params),
  getApplyProgress: (params: { gameId: string }) =>
    invoke<ApplyProgressSnapshot | null>('get_apply_progress', params),
  listCollections: (params: { gameId: string; isSafe: boolean }) =>
    invoke<CollectionSummary[]>('list_collections', params),
  getCollectionPreview: (params: { id?: string; collectionId?: string; gameId?: string }) =>
    invoke<CollectionPreview>('get_collection_preview', params),
  previewApplyCollection: (params: {
    id?: string;
    collectionId?: string;
    gameId?: string;
    isSafe?: boolean;
  }) => invoke<ApplyPreview>('preview_apply_collection', params),
  createCollection: (params: {
    gameId?: string;
    name: string;
    saveMode?: 'save_current_state' | 'clone_snapshot';
    sourceCollectionId?: string | null;
    description?: string;
  }) => invoke<CollectionSummary>('create_collection', params),
  updateCollection: (params: { id?: string; gameId?: string; name?: string }) =>
    invoke<CollectionSummary>('update_collection', params),
  deleteCollection: (params: { id?: string; collectionId?: string; gameId?: string }) =>
    invoke<void>('delete_collection', params),
  applyCollection: (params: {
    id?: string;
    collectionId?: string;
    gameId?: string;
    ignoreMissing?: boolean;
  }) => invoke<ApplyResult>('apply_collection', params),
  switchCorridor: (params: { gameId?: string; targetSafe?: boolean }) =>
    invoke<SwitchResult>('switch_corridor', params),
  previewCorridorSwitch: (params: { gameId?: string; targetSafe?: boolean }) =>
    invoke<CorridorSwitchPreview>('preview_corridor_switch', params),
  clearPendingTasks: (params?: { gameId?: string }) => invoke<void>('clear_pending_tasks', params),
  resolveRecoveryTask: (params: { taskId: string; action: RecoveryAction }) =>
    invoke<void>('resolve_recovery_task', params),

  // Security (PIN)
  hasPin: () => invoke<boolean>('has_pin'),
  setPin: (params: { pin: string; recoveryCode?: string }) => invoke<void>('set_pin', params),
  verifyPin: (params: { pin?: string }) => invoke<boolean>('verify_pin', params),
  clearPin: (params: { pin?: string }) => invoke<void>('clear_pin', params),
  getPinStatus: () => invoke<PinStatus>('get_pin_status'),
  resetPinWithRecoveryCode: (params: { code?: string; recoveryCode?: string }) =>
    invoke<boolean>('reset_pin_with_recovery_code', params),

  // Browser
  browserOpenTab: (params: { url: string }) => invoke<string>('browser_open_tab', params),
  browserNavigate: (params: { id?: string; url: string; label?: string }) =>
    invoke<void>('browser_navigate', params),
  browserReloadTab: (params: { id?: string; label?: string }) =>
    invoke<void>('browser_reload_tab', params),
  browserCloseTab: (params: { id?: string; label?: string }) =>
    invoke<void>('browser_close_tab', params),
  browserClearData: (params: { id?: string; label?: string }) =>
    invoke<void>('browser_clear_data', params),
  browserGetHomepage: () => invoke<string>('browser_get_homepage'),
  browserSetHomepage: (params: { url: string }) => invoke<void>('browser_set_homepage', params),
  browserImportSelected: (params: { id?: string; ids?: string[]; gameId?: string }) =>
    invoke<void>('browser_import_selected', params),
  browserListDownloads: () => invoke<BrowserDownloadItem[]>('browser_list_downloads'),
  browserDeleteDownload: (params: { id?: string; ids?: string[]; deleteFile?: boolean }) =>
    invoke<void>('browser_delete_download', params),
  browserCancelDownload: (params: { id?: string; ids?: string[]; deleteFile?: boolean }) =>
    invoke<void>('browser_cancel_download', params),
  browserClearImported: () => invoke<void>('browser_clear_imported'),
  browserClearOldDownloads: () => invoke<void>('browser_clear_old_downloads'),
  browserListImportQueue: () => invoke<ImportJobItem[]>('browser_list_import_queue'),
  browserConfirmImport: (params: {
    id?: string;
    jobId?: string;
    gameId?: string;
    category?: string;
    objectId?: string;
  }) => invoke<void>('browser_confirm_import', params),
  browserCancelImport: (params: { id?: string; jobId?: string }) =>
    invoke<void>('browser_cancel_import', params),

  // Trash
  listTrash: () => invoke<TrashMetadata[]>('list_trash'),
  emptyTrash: () => invoke<number>('empty_trash'),
  restoreMod: (params: { trashId: string; gameId?: string }) => invoke<void>('restore_mod', params),

  // Themes
  listCustomThemes: () => invoke<ThemeMetadata[]>('list_custom_themes'),
  loadCustomTheme: (params: { id: string }) => invoke<CustomTheme>('load_custom_theme', params),
  saveCustomTheme: (params: { theme: CustomTheme }) => invoke<void>('save_custom_theme', params),
  deleteCustomTheme: (params: { id: string }) => invoke<void>('delete_custom_theme', params),
};
